#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crossterm::event::read;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use std::ffi::CStr;
use std::time::Instant;
use std::{ffi::CString, slice, time::Duration};
use terminal_window::{Pixel, TerminalWindow};

mod terminal_window;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

const FPS: usize = 60;

extern "C" fn software_surface_present_callback(
    user_data: *mut std::os::raw::c_void,
    allocation: *const std::os::raw::c_void,
    row_bytes: usize,
    height: usize,
) -> bool {
    let allocation: &[u8] =
        unsafe { slice::from_raw_parts(allocation as *const u8, row_bytes * height) };

    let user_data: &mut TerminalEmbedder = unsafe { std::mem::transmute(user_data) };
    assert_eq!(
        user_data.corruption_token, "user_data",
        "not corrupt in software callback"
    );

    let terminal_window = &mut user_data.terminal;
    assert_eq!(
        terminal_window.corruption_token, "terminal",
        "not corrupt in software callback"
    );

    // In allocation, each group of 4 bits represents a pixel. In order, each of
    // the 4 bits will be [b, g, r, a].
    let buf = allocation
        .chunks(4)
        .into_iter()
        .map(|c| {
            let b = c[0];
            let g = c[1];
            let r = c[2];
            let a = c[3];

            Pixel { r, g, b, a }
        })
        .collect::<Vec<Pixel>>();

    /*
      bytes / row = row_bytes
      elements / byte = 1

      1 pixel = 4 elements
      elements / pixel = 1/4
      width = pixels / row = pixels * (elements / pixel) / row = 1/4 * elements / row

      width = 1/4 * (elements / byte) * (bytes / row)
      width = 1/4 * 1 * row_bytes
      width = row_bytes / 4
    */
    let width = row_bytes / 4;

    user_data.draw(width, height, buf);

    return true;
}

struct ProjectArgs {
    assets_path: *mut i8,
    icu_data_path: *mut i8,
}

impl ProjectArgs {
    fn new(assets_path: &str, icu_data_path: &str) -> Self {
        let assets_path = CString::new(assets_path).unwrap().into_raw();
        let icu_data_path = CString::new(icu_data_path).unwrap().into_raw();

        Self {
            assets_path,
            icu_data_path,
        }
    }
}

impl Drop for ProjectArgs {
    fn drop(&mut self) {
        unsafe {
            let _ = CString::from_raw(self.assets_path);
            let _ = CString::from_raw(self.icu_data_path);
        }
    }
}

impl From<&ProjectArgs> for FlutterProjectArgs {
    fn from(value: &ProjectArgs) -> Self {
        FlutterProjectArgs {
            struct_size: std::mem::size_of::<FlutterProjectArgs>(),
            assets_path: value.assets_path,
            main_path__unused__: std::ptr::null(),
            packages_path__unused__: std::ptr::null(),
            icu_data_path: value.icu_data_path,
            command_line_argc: 0,
            command_line_argv: std::ptr::null(),
            platform_message_callback: None,
            vm_snapshot_data: std::ptr::null(),
            vm_snapshot_data_size: 0,
            vm_snapshot_instructions: std::ptr::null(),
            vm_snapshot_instructions_size: 0,
            isolate_snapshot_data: std::ptr::null(),
            isolate_snapshot_data_size: 0,
            isolate_snapshot_instructions: std::ptr::null(),
            isolate_snapshot_instructions_size: 0,
            root_isolate_create_callback: None,
            update_semantics_node_callback: None,
            update_semantics_custom_action_callback: None,
            persistent_cache_path: std::ptr::null(),
            is_persistent_cache_read_only: false,
            vsync_callback: None,
            custom_dart_entrypoint: std::ptr::null(),
            custom_task_runners: std::ptr::null(),
            shutdown_dart_vm_when_done: true,
            compositor: std::ptr::null(),
            dart_old_gen_heap_size: 0,
            aot_data: std::ptr::null_mut(),
            compute_platform_resolved_locale_callback: None,
            dart_entrypoint_argc: 0,
            dart_entrypoint_argv: std::ptr::null(),
            log_message_callback: Some(log_message_callback),
            log_tag: std::ptr::null(),
            on_pre_engine_restart_callback: None,
            update_semantics_callback: None,
        }
    }
}

trait Engine {
    fn log(&self, tag: String, message: String);

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>);
}

extern "C" fn log_message_callback(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &mut TerminalEmbedder = unsafe { std::mem::transmute(user_data) };
    let tag = to_string(tag);
    let message = to_string(message);

    user_data.log(tag, message);
}

fn to_string(c_str: *const std::os::raw::c_char) -> String {
    let message = unsafe { CStr::from_ptr(c_str) };
    let message = message.to_owned();

    message.to_str().unwrap().to_string()
}

pub struct SafeEngine {
    engine: FlutterEngine,
    user_data: *mut TerminalEmbedder,
    engine_start_time: Duration,
    start_instant: Instant,
}

struct TerminalEmbedder {
    terminal: TerminalWindow,
    corruption_token: String,
}

impl Engine for TerminalEmbedder {
    fn log(&self, tag: String, message: String) {
        // TODO: Print to the main terminal.
        println!("{tag}: {message}");
    }

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) {
        self.terminal.draw(width, height, buffer).unwrap();
    }
}

impl Drop for SafeEngine {
    fn drop(&mut self) {
        unsafe { FlutterEngineShutdown(self.engine) };
        unsafe { Box::from_raw(self.user_data) };
    }
}

impl SafeEngine {
    pub fn new(assets_dir: &str, icu_data_path: &str) -> Self {
        let renderer_config = FlutterRendererConfig {
            type_: FlutterRendererType_kSoftware,
            __bindgen_anon_1: FlutterRendererConfig__bindgen_ty_1 {
                software: FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<FlutterSoftwareRendererConfig>(),
                    surface_present_callback: Some(software_surface_present_callback),
                },
            },
        };

        let project_args = ProjectArgs::new(assets_dir, icu_data_path);

        let embedder = Self {
            engine: std::ptr::null_mut(),
            // `UserData` needs to be on the heap so that the Flutter Engine
            // callbacks can safely provide a pointer to it (if it was on the
            // stack, there is a chance that the value is dropped when the
            // callbacks still reference it). So opt into manual memory
            // management of this struct.
            user_data: Box::into_raw(
                TerminalEmbedder {
                    terminal: TerminalWindow::new("terminal".to_string()),
                    corruption_token: "user_data".to_string(),
                }
                .into(),
            ),

            engine_start_time: Duration::from_nanos(unsafe { FlutterEngineGetCurrentTime() }),
            start_instant: Instant::now(),
        };

        let user_data_ptr = embedder.user_data;

        assert_eq!(
            unsafe {
                FlutterEngineRun(
                    1,
                    &renderer_config,
                    &FlutterProjectArgs::from(&project_args) as *const FlutterProjectArgs,
                    user_data_ptr as *mut std::ffi::c_void,
                    &embedder.engine as *const FlutterEngine as *mut FlutterEngine,
                )
            },
            FlutterEngineResult_kSuccess,
            "Engine started successfully"
        );

        let display = FlutterEngineDisplay {
            struct_size: std::mem::size_of::<FlutterEngineDisplay>(),
            display_id: 0,
            single_display: true,
            refresh_rate: FPS as f64,
        };

        assert_eq!(
            unsafe {
                FlutterEngineNotifyDisplayUpdate(
                    embedder.engine,
                    FlutterEngineDisplaysUpdateType_kFlutterEngineDisplaysUpdateTypeStartup,
                    &display as *const FlutterEngineDisplay,
                    1,
                )
            },
            FlutterEngineResult_kSuccess,
            "notify display update"
        );

        let s = unsafe { &*embedder.user_data };
        let (width, height) = s.terminal.size();

        let event = FlutterWindowMetricsEvent {
            struct_size: std::mem::size_of::<FlutterWindowMetricsEvent>(),
            width,
            height,
            pixel_ratio: 1.0,
            left: 0,
            top: 0,
            physical_view_inset_top: 0.0,
            physical_view_inset_right: 0.0,
            physical_view_inset_bottom: 0.0,
            physical_view_inset_left: 0.0,
        };
        assert_eq!(
            unsafe {
                FlutterEngineSendWindowMetricsEvent(
                    embedder.engine,
                    &event as *const FlutterWindowMetricsEvent,
                )
            },
            FlutterEngineResult_kSuccess,
            "Window metrics set successfully"
        );

        embedder
    }

    /// Returns a duration from when the Flutter Engine was started.
    fn duration_from_start(&self) -> Duration {
        // Always offset instants from `engine_start_time` to match the engine time base.
        Instant::now().duration_since(self.start_instant) + self.engine_start_time
    }

    pub fn wait_for_input(&self) {
        loop {
            match read().unwrap() {
                crossterm::event::Event::FocusGained => todo!(),
                crossterm::event::Event::FocusLost => todo!(),
                crossterm::event::Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => {
                    if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
                        break;
                    }
                }
                crossterm::event::Event::Mouse(MouseEvent {
                    kind,
                    column,
                    row,
                    modifiers: _,
                }) => {
                    // The terminal renderer merges two pixels (top and bottom) into one.
                    let row = row * 2;

                    let (phase, buttons) = match kind {
                        crossterm::event::MouseEventKind::Down(mouse_button) => (
                            FlutterPointerPhase_kDown,
                            to_flutter_mouse_button(mouse_button),
                        ),
                        crossterm::event::MouseEventKind::Up(mouse_button) => (
                            FlutterPointerPhase_kUp,
                            to_flutter_mouse_button(mouse_button),
                        ),
                        // Just continue as it's too annoying to log these common events.
                        crossterm::event::MouseEventKind::Drag(_) => continue,
                        crossterm::event::MouseEventKind::Moved => continue,
                        kind => {
                            println!("ignoring event {kind:?}");
                            continue;
                        }
                    };

                    let flutter_pointer_event = FlutterPointerEvent {
                        struct_size: std::mem::size_of::<FlutterPointerEvent>(),
                        phase,
                        timestamp: self.duration_from_start().as_micros() as usize,
                        x: column as f64,
                        y: row as f64,
                        device: 0,
                        signal_kind: 0,
                        scroll_delta_x: 0.0,
                        scroll_delta_y: 0.0,
                        device_kind: FlutterPointerDeviceKind_kFlutterPointerDeviceKindMouse,
                        // This is probably a bitmask for multiple buttons so the
                        // type doesn't match.
                        buttons: buttons as i64,
                        pan_x: 0.0,
                        pan_y: 0.0,
                        scale: 0.0,
                        rotation: 0.0,
                    };

                    unsafe {
                        assert_eq!(
                            FlutterEngineSendPointerEvent(self.engine, &flutter_pointer_event, 1),
                            FlutterEngineResult_kSuccess
                        );
                        assert_eq!(
                            FlutterEngineScheduleFrame(self.engine),
                            FlutterEngineResult_kSuccess
                        );
                    }
                }
                crossterm::event::Event::Paste(_) => todo!(),
                crossterm::event::Event::Resize(columns, rows) => {
                    let event = FlutterWindowMetricsEvent {
                        struct_size: std::mem::size_of::<FlutterWindowMetricsEvent>(),
                        width: columns as usize,
                        // The terminal renderer merges two pixels (top and bottom) into one.
                        height: (rows * 2) as usize,
                        pixel_ratio: 1.0,
                        left: 0,
                        top: 0,
                        physical_view_inset_top: 0.0,
                        physical_view_inset_right: 0.0,
                        physical_view_inset_bottom: 0.0,
                        physical_view_inset_left: 0.0,
                    };
                    assert_eq!(
                        unsafe {
                            FlutterEngineSendWindowMetricsEvent(
                                self.engine,
                                &event as *const FlutterWindowMetricsEvent,
                            )
                        },
                        FlutterEngineResult_kSuccess,
                        "Window metrics set successfully"
                    );
                }
            }
        }
    }
}

fn to_flutter_mouse_button(button: MouseButton) -> FlutterPointerMouseButtons {
    match button {
        crossterm::event::MouseButton::Left => {
            FlutterPointerMouseButtons_kFlutterPointerButtonMousePrimary
        }
        crossterm::event::MouseButton::Right => {
            FlutterPointerMouseButtons_kFlutterPointerButtonMouseSecondary
        }
        crossterm::event::MouseButton::Middle => {
            FlutterPointerMouseButtons_kFlutterPointerButtonMouseMiddle
        }
    }
}
