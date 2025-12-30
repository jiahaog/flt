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
use metal::{Device, MTLPixelFormat, MTLRegion, MTLTextureUsage, TextureDescriptor};
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
            // Try to initialize Metal
            if let Some(device) = Device::system_default() {
                let command_queue = device.new_command_queue();
                let device = Arc::new(device);
                let current_texture = Arc::new(Mutex::new(None::<metal::Texture>));

                let device_clone = device.clone();
                let queue_clone = command_queue.clone();
                let texture_clone = current_texture.clone();
                let present_texture_clone = current_texture.clone();

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
                            descriptor.set_storage_mode(metal::MTLStorageMode::Managed);

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
                            let width = texture.width() as usize;
                            let height = texture.height() as usize;
                            let row_bytes = width * 4;
                            let length = row_bytes * height;

                            // Synchronize the texture for CPU access.
                            //
                            // On macOS with `MTLStorageModeManaged`, the texture data exists in two places:
                            // video memory (VRAM) for the GPU and system memory (RAM) for the CPU.
                            // When Flutter renders, it writes to VRAM. To read these pixels back to the CPU
                            // (for our terminal display), we must explicitly trigger a synchronization.
                            //
                            // 1. `synchronize_resource`: Enqueues a blit command to copy the latest VRAM content
                            //    to the CPU-accessible RAM buffer.
                            // 2. `wait_until_completed`: Blocks the current thread until the GPU has finished
                            //    processing all commands, ensuring the data in RAM is fully updated and valid
                            //    before we attempt to read it with `get_bytes`.
                            //
                            // Without this synchronization, `get_bytes` would read stale or incomplete data from
                            // system RAM, leading to severe graphical artifacts, flickering (seeing the previous frame),
                            // or screen tearing.
                            let command_buffer = queue_clone.new_command_buffer();
                            let blit_encoder = command_buffer.new_blit_command_encoder();
                            blit_encoder.synchronize_resource(texture);
                            blit_encoder.end_encoding();

                            command_buffer.commit();
                            command_buffer.wait_until_completed();

                            let mut buffer = vec![0u8; length];
                            texture.get_bytes(
                                buffer.as_mut_ptr() as *mut c_void,
                                row_bytes as u64,
                                MTLRegion::new_2d(0, 0, width as u64, height as u64),
                                0,
                            );

                            sender_d
                                .send(PlatformEvent::EngineEvent(EngineEvent::Draw(
                                    buffer, width, height,
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
