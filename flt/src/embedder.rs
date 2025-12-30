use crate::constants::FPS;
use crate::event::{EngineEvent, PlatformEvent};
use crate::semantics::FlutterSemanticsTree;
use crate::task_runner::TaskRunner;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{sys, Callbacks, FlutterEngine};
#[cfg(target_os = "macos")]
use metal::foreign_types::ForeignType;
#[cfg(target_os = "macos")]
use metal::{Device, MTLPixelFormat, MTLTextureUsage, TextureDescriptor};
use std::ffi::c_void;
use std::mem;
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;

pub struct TerminalEmbedder {
    pub(crate) engine: FlutterEngine,
    pub(crate) semantics_tree: FlutterSemanticsTree,
    pub(crate) terminal_window: TerminalWindow,

    // Switches provided at startup.
    // TODO(jiahaog): This should be a path instead.
    pub(crate) debug_semantics: bool,
    pub(crate) show_semantics: bool,

    // Event related.
    pub(crate) should_run: bool,
    pub(crate) platform_events: Receiver<PlatformEvent>,
    pub(crate) platform_task_runner: TaskRunner,

    // Window related.
    pub(crate) dimensions: (usize, usize),
    pub(crate) zoom: f64,
    pub(crate) scale: f64,
    pub(crate) window_offset: (isize, isize),
    pub(crate) prev_window_offset: (isize, isize),
    pub(crate) mouse_down_pos: (isize, isize),
}

impl TerminalEmbedder {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        simple_output: bool,
        alternate_screen: bool,
        log_events: bool,
        debug_semantics: bool,
        kitty_mode: bool,
    ) -> Result<Self, Error> {
        let (main_sender, main_receiver) = channel();

        let terminal_window = TerminalWindow::new(
            simple_output,
            alternate_screen,
            log_events,
            kitty_mode,
            main_sender.clone(),
        );

        let (sender_a, sender_b, sender_c, sender_d, sender_e) = (
            main_sender.clone(),
            main_sender.clone(),
            main_sender.clone(),
            main_sender.clone(),
            main_sender.clone(),
        );
        let platform_thread_id = thread::current().id();

        // Common callback logic
        // We box these up to share them between the Metal and Software paths easily
        // (or rather, to avoid code duplication in defining them).
        // Since Callbacks requires Box<dyn Fn...>, we can create them here.

        let post_platform_task_callback: Box<dyn Fn(flutter_sys::EngineTask)> =
            Box::new(move |task| {
                sender_a
                    .send(PlatformEvent::EngineEvent(EngineEvent::EngineTask(task)))
                    .unwrap();
            });

        let platform_task_runs_task_on_current_thread_callback: Box<dyn Fn() -> bool> =
            Box::new(move || thread::current().id() == platform_thread_id);

        let log_message_callback: Box<dyn Fn(String, String)> = Box::new(move |tag, message| {
            sender_b
                .send(PlatformEvent::EngineEvent(EngineEvent::LogMessage {
                    tag,
                    message,
                }))
                .unwrap();
        });

        let update_semantics_callback: Box<dyn Fn(Vec<flutter_sys::SemanticsUpdate>)> =
            Box::new(move |updates| {
                sender_c
                    .send(PlatformEvent::EngineEvent(EngineEvent::UpdateSemantics(
                        updates,
                    )))
                    .unwrap();
            });

        let platform_message_callback: Box<dyn Fn(flutter_sys::PlatformMessage)> =
            Box::new(move |message| {
                sender_e
                    .send(PlatformEvent::EngineEvent(EngineEvent::PlatformMessage(
                        message,
                    )))
                    .unwrap();
            });

        // This draw callback is ONLY used for software rendering.
        // For Metal, we use present_drawable_callback.
        let sender_d_software = sender_d.clone();
        let software_draw_callback: Box<dyn Fn(&[u8], usize, usize)> =
            Box::new(move |buffer, width, height| {
                sender_d_software
                    .send(PlatformEvent::EngineEvent(EngineEvent::Draw(
                        buffer.to_vec(),
                        width,
                        height,
                    )))
                    .unwrap();
            });

        // Initialize Callbacks and Renderer resources based on platform

        #[cfg(target_os = "macos")]
        let (callbacks, device_ptr, queue_ptr) = {
            const SHADERS_SOURCE: &str = r#"
#include <metal_stdlib>
using namespace metal;

// Why this shader exists:
// 1. The Kitty Graphics Protocol (f=32) strictly requires RGBA pixel data.
// 2. Flutter's Metal backend (via Skia) is hardcoded to output BGRA on macOS.
//    See: flutter/engine/src/flutter/shell/gpu/gpu_surface_metal_skia.mm
//    specifically the usage of `kBGRA_8888_SkColorType`.
// 3. We cannot change the texture format to RGBA because Skia will fail to create a surface.
// 4. Swapping BGRA -> RGBA on the CPU (iterating over every pixel) is extremely slow.
//
// Solution: Use this Compute Shader to efficiently read the BGRA texture and write RGBA bytes
// to a shared buffer on the GPU.
kernel void copy_texture_to_buffer(texture2d<float, access::read> inputTexture [[texture(0)]],
                                    device uchar4 *outputBuffer [[buffer(0)]],
                                    uint2 gid [[thread_position_in_grid]]) {

    if (gid.x >= inputTexture.get_width() || gid.y >= inputTexture.get_height()) {
        return;
    }

    float4 color = inputTexture.read(gid);
    uint index = gid.y * inputTexture.get_width() + gid.x;
    outputBuffer[index] = uchar4(color.r * 255.0, color.g * 255.0, color.b * 255.0, color.a * 255.0);
}
                    "#;

            // Try to initialize Metal
            if let Some(device) = Device::system_default() {
                let command_queue = device.new_command_queue();

                let device = Arc::new(device);

                // Compile Shader

                let library = device
                    .new_library_with_source(SHADERS_SOURCE, &metal::CompileOptions::new())
                    .unwrap();
                let function = library
                    .get_function("copy_texture_to_buffer", None)
                    .unwrap();
                let compute_pipeline = device
                    .new_compute_pipeline_state_with_function(&function)
                    .unwrap();

                let compute_pipeline = Arc::new(compute_pipeline);

                let current_texture = Arc::new(Mutex::new(None::<metal::Texture>));

                // Persistent output buffer (Shared memory)
                let output_buffer = Arc::new(Mutex::new(None::<metal::Buffer>));

                let device_clone = device.clone();
                let device_for_present = device.clone();
                let queue_clone = command_queue.clone();
                let texture_clone = current_texture.clone();
                let present_texture_clone = current_texture.clone();
                let output_buffer_clone = output_buffer.clone();
                let compute_pipeline_clone = compute_pipeline.clone();

                let callbacks = Callbacks {
                    post_platform_task_callback: Some(post_platform_task_callback),

                    platform_task_runs_task_on_current_thread_callback: Some(
                        platform_task_runs_task_on_current_thread_callback,
                    ),

                    log_message_callback: Some(log_message_callback),

                    update_semantics_callback: Some(update_semantics_callback),

                    platform_message_callback: Some(platform_message_callback),

                    draw_callback: None, // Metal doesn't use this

                    get_next_drawable_callback: Some(Box::new(move |frame_info| {
                        let width = frame_info.size.width as u64;
                        let height = frame_info.size.height as u64;

                        let mut tex_guard = texture_clone.lock().unwrap();

                        let needs_new_texture = if let Some(tex) = &*tex_guard {
                            tex.width() != width || tex.height() != height
                        } else {
                            true
                        };

                        if needs_new_texture {
                            let descriptor = TextureDescriptor::new();
                            descriptor.set_pixel_format(MTLPixelFormat::BGRA8Unorm);
                            descriptor.set_width(width);
                            descriptor.set_height(height);
                            descriptor.set_usage(
                                MTLTextureUsage::RenderTarget | MTLTextureUsage::ShaderRead,
                            );
                            descriptor.set_storage_mode(metal::MTLStorageMode::Private);

                            let texture = device_clone.new_texture(&descriptor);
                            *tex_guard = Some(texture);
                        }

                        let texture_ref = tex_guard.as_ref().unwrap();
                        let texture_ptr = texture_ref.as_ptr() as *const c_void;

                        sys::FlutterMetalTexture {
                            struct_size: mem::size_of::<sys::FlutterMetalTexture>(),
                            texture_id: 1,
                            texture: texture_ptr,
                            user_data: std::ptr::null_mut(),
                            destruction_callback: None,
                        }
                    })),
                    present_drawable_callback: Some(Box::new(move |_texture_wrapper| {
                        let tex_guard = present_texture_clone.lock().unwrap();
                        if let Some(texture) = &*tex_guard {
                            let width = texture.width();
                            let height = texture.height();
                            let length = (width * height * 4) as u64;
                            let command_buffer = queue_clone.new_command_buffer();

                            // Setup Output Buffer

                            let mut buf_guard = output_buffer_clone.lock().unwrap();

                            if buf_guard
                                .as_ref()
                                .map(|b| b.length() != length)
                                .unwrap_or(true)
                            {
                                let new_buf = device_for_present.new_buffer(
                                    length,
                                    metal::MTLResourceOptions::StorageModeShared,
                                );
                                *buf_guard = Some(new_buf);
                            }

                            let buffer = buf_guard.as_ref().unwrap();

                            // Encode Compute Command

                            let encoder = command_buffer.new_compute_command_encoder();
                            encoder.set_compute_pipeline_state(&compute_pipeline_clone);
                            encoder.set_texture(0, Some(texture));
                            encoder.set_buffer(0, Some(buffer), 0);

                            let w = compute_pipeline_clone.thread_execution_width();
                            let h = compute_pipeline_clone.max_total_threads_per_threadgroup() / w;

                            let threads_per_group = metal::MTLSize::new(w, h, 1);
                            let threads_per_grid = metal::MTLSize::new(width, height, 1);

                            encoder.dispatch_threads(threads_per_grid, threads_per_group);
                            encoder.end_encoding();

                            // Synchronize the buffer for CPU access.
                            //
                            // On macOS with `MTLStorageModeManaged` (Intel), the buffer data exists in two places:
                            // VRAM (GPU) and RAM (CPU). The Compute Shader writes to VRAM.
                            // To read these pixels back to the CPU, we must explicitly trigger a synchronization.
                            //
                            // 1. `synchronize_resource`: Enqueues a blit command to copy the latest VRAM content
                            //    to the CPU-accessible RAM buffer.
                            // 2. `wait_until_completed`: Blocks the current thread until the GPU has finished
                            //    processing all commands, ensuring the data in RAM is fully updated and valid.
                            //
                            // For `MTLStorageModeShared` (Apple Silicon), this synchronization is implicit or
                            // efficient, but the command structure remains correct for compatibility.
                            if buffer.storage_mode() == metal::MTLStorageMode::Managed {
                                let blit = command_buffer.new_blit_command_encoder();
                                blit.synchronize_resource(buffer);
                                blit.end_encoding();
                            }

                            command_buffer.commit();
                            command_buffer.wait_until_completed();

                            // Read Data
                            let ptr = buffer.contents() as *const u8;

                            // Create a copy to send
                            let data = unsafe { std::slice::from_raw_parts(ptr, length as usize) }
                                .to_vec();

                            sender_d
                                .send(PlatformEvent::EngineEvent(EngineEvent::Draw(
                                    data,
                                    width as usize,
                                    height as usize,
                                )))
                                .unwrap();
                            true
                        } else {
                            false
                        }
                    })),
                };

                // Prevent dropping the device/queue as engine holds raw pointers
                let device_ptr = device.as_ptr() as *mut c_void;
                let queue_ptr = command_queue.as_ptr() as *mut c_void;
                mem::forget(device);
                mem::forget(command_queue);

                (callbacks, Some(device_ptr), Some(queue_ptr))
            } else {
                eprintln!("Metal is not available. Falling back to software rendering.");
                // Fallback to Software
                let callbacks = Callbacks {
                    post_platform_task_callback: Some(post_platform_task_callback),
                    platform_task_runs_task_on_current_thread_callback: Some(
                        platform_task_runs_task_on_current_thread_callback,
                    ),
                    log_message_callback: Some(log_message_callback),
                    update_semantics_callback: Some(update_semantics_callback),
                    platform_message_callback: Some(platform_message_callback),
                    draw_callback: Some(software_draw_callback),
                    get_next_drawable_callback: None,
                    present_drawable_callback: None,
                };
                (callbacks, None, None)
            }
        };

        #[cfg(not(target_os = "macos"))]
        let (callbacks, device_ptr, queue_ptr) = {
            let callbacks = Callbacks {
                post_platform_task_callback: Some(post_platform_task_callback),
                platform_task_runs_task_on_current_thread_callback: Some(
                    platform_task_runs_task_on_current_thread_callback,
                ),
                log_message_callback: Some(log_message_callback),
                update_semantics_callback: Some(update_semantics_callback),
                platform_message_callback: Some(platform_message_callback),
                draw_callback: Some(software_draw_callback),
                get_next_drawable_callback: None,
                present_drawable_callback: None,
            };
            (callbacks, None, None)
        };

        let (width, height) = terminal_window.size();

        let mut embedder = Self {
            engine: FlutterEngine::new(
                assets_dir,
                icu_data_path,
                callbacks,
                device_ptr,
                queue_ptr,
            )?,
            terminal_window,
            semantics_tree: FlutterSemanticsTree::new(),
            debug_semantics,
            show_semantics: false,
            should_run: true,
            platform_events: main_receiver,
            platform_task_runner: TaskRunner::new(),
            dimensions: (0, 0),
            zoom: 1.0,
            scale: 1.0,
            window_offset: (0, 0),
            prev_window_offset: (0, 0),
            mouse_down_pos: (0, 0),
        };

        embedder.engine.notify_display_update(
            FPS as f64,
            (width, height),
            embedder.terminal_window.device_pixel_ratio(),
        )?;
        embedder.reset_viewport()?;

        // This event sets the engine window dimensions which will kickstart rendering.
        main_sender
            .send(PlatformEvent::EngineEvent(EngineEvent::Draw(vec![], 0, 0)))
            .unwrap();

        Ok(embedder)
    }

    pub(crate) fn reset_viewport(&mut self) -> Result<(), Error> {
        self.dimensions = self.terminal_window.size();
        self.zoom = 1.0;
        self.scale = 1.0;
        self.window_offset = (0, 0);
        self.prev_window_offset = (0, 0);
        self.mouse_down_pos = (0, 0);

        self.terminal_window.mark_dirty();
        self.engine.schedule_frame()?;
        Ok(())
    }
}
