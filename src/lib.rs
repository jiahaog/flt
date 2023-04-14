#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crossterm::event::read;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use engine::Pixel;
use std::cell::RefCell;
use std::ffi::CStr;
use std::rc::Rc;
use std::time::Instant;
use std::{ffi::CString, slice, time::Duration};
use terminal_window::TerminalWindow;

mod engine;
mod terminal_window;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

const FPS: usize = 60;

extern "C" fn software_surface_present_callback<T: Embedder>(
    user_data: *mut std::os::raw::c_void,
    allocation: *const std::os::raw::c_void,
    row_bytes: usize,
    height: usize,
) -> bool {
    let allocation: &[u8] =
        unsafe { slice::from_raw_parts(allocation as *const u8, row_bytes * height) };

    let user_data: &mut T = unsafe { std::mem::transmute(user_data) };

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

extern "C" fn log_message_callback(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &mut TerminalEmbedderImpl = unsafe { std::mem::transmute(user_data) };
    let tag = to_string(tag);
    let message = to_string(message);

    user_data.log(tag, message);
}

fn to_string(c_str: *const std::os::raw::c_char) -> String {
    let message = unsafe { CStr::from_ptr(c_str) };
    let message = message.to_owned();

    message.to_str().unwrap().to_string()
}

struct SafeEngine<T: Embedder> {
    engine: FlutterEngine,
    user_data: *mut T,
    engine_start_time: Duration,
    start_instant: Instant,
}

struct TerminalEmbedderImpl {
    outside: Rc<RefCell<TerminalWindow>>,
}
pub trait Embedder {
    fn log(&self, tag: String, message: String);

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>);

    // TODO(jiahaog): Remove. The Terminal Embedder should have methods to set
    // the size on the engine.
    fn size(&self) -> (usize, usize);
}

pub struct TerminalEmbedder {
    corruption_token: String,
    engine: SafeEngine<TerminalEmbedderImpl>,
    terminal_window: Rc<RefCell<TerminalWindow>>,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str) -> Self {
        let terminal_window = Rc::new(RefCell::new(TerminalWindow::new("terminal".to_string())));

        let embedder = TerminalEmbedderImpl {
            outside: terminal_window.clone(),
        };

        let terminal_embedder = Self {
            corruption_token: "user_data".to_string(),
            terminal_window: terminal_window.clone(),
            engine: SafeEngine::new(assets_dir, icu_data_path, embedder),
        };

        terminal_embedder.engine.notify_display_update(FPS as f64);

        let (width, height) = terminal_window.borrow().size();
        terminal_embedder
            .engine
            .send_window_metrics_event(width, height);

        terminal_embedder
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

                    let (phase, button) = match kind {
                        crossterm::event::MouseEventKind::Down(mouse_button) => {
                            (SafePointerPhase::Down, mouse_button.into())
                        }
                        crossterm::event::MouseEventKind::Up(mouse_button) => {
                            (SafePointerPhase::Up, mouse_button.into())
                        }
                        // Just continue as it's too annoying to log these common events.
                        crossterm::event::MouseEventKind::Drag(_) => continue,
                        crossterm::event::MouseEventKind::Moved => continue,
                        kind => {
                            println!("ignoring event {kind:?}");
                            continue;
                        }
                    };

                    self.engine
                        .send_pointer_event(phase, column as f64, row as f64, vec![button]);
                }
                crossterm::event::Event::Paste(_) => todo!(),
                crossterm::event::Event::Resize(columns, rows) => {
                    self.engine.send_window_metrics_event(
                        columns as usize,
                        // The terminal renderer merges two pixels (top and bottom) into one.
                        (rows * 2) as usize,
                    );
                }
            }
        }
    }
}

impl Embedder for TerminalEmbedderImpl {
    fn log(&self, tag: String, message: String) {
        // TODO: Print to the main terminal.
        println!("{tag}: {message}");
    }

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) {
        self.outside
            .borrow_mut()
            .draw(width, height, buffer)
            .unwrap()
    }

    fn size(&self) -> (usize, usize) {
        self.outside.borrow().size()
    }
}

impl<T: Embedder> Drop for SafeEngine<T> {
    fn drop(&mut self) {
        unsafe { FlutterEngineShutdown(self.engine) };
        unsafe { Box::from_raw(self.user_data) };
    }
}

impl<T: Embedder> SafeEngine<T> {
    fn new(assets_dir: &str, icu_data_path: &str, embedder: T) -> Self {
        let renderer_config = FlutterRendererConfig {
            type_: FlutterRendererType_kSoftware,
            __bindgen_anon_1: FlutterRendererConfig__bindgen_ty_1 {
                software: FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<FlutterSoftwareRendererConfig>(),
                    surface_present_callback: Some(software_surface_present_callback::<T>),
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
            user_data: Box::into_raw(embedder.into()),

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

        embedder
    }

    /// Returns a duration from when the Flutter Engine was started.
    fn duration_from_start(&self) -> Duration {
        // Always offset instants from `engine_start_time` to match the engine time base.
        Instant::now().duration_since(self.start_instant) + self.engine_start_time
    }

    fn notify_display_update(&self, refresh_rate: f64) {
        let display = FlutterEngineDisplay {
            struct_size: std::mem::size_of::<FlutterEngineDisplay>(),
            display_id: 0,
            single_display: true,
            refresh_rate,
        };

        assert_eq!(
            unsafe {
                FlutterEngineNotifyDisplayUpdate(
                    self.engine,
                    FlutterEngineDisplaysUpdateType_kFlutterEngineDisplaysUpdateTypeStartup,
                    &display as *const FlutterEngineDisplay,
                    1,
                )
            },
            FlutterEngineResult_kSuccess,
            "notify display update"
        );
    }

    fn send_window_metrics_event(&self, width: usize, height: usize) {
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
                    self.engine,
                    &event as *const FlutterWindowMetricsEvent,
                )
            },
            FlutterEngineResult_kSuccess,
            "Window metrics set successfully"
        );
    }

    fn send_pointer_event(
        &self,
        phase: SafePointerPhase,
        x: f64,
        y: f64,
        buttons: Vec<SafeMouseButton>,
    ) {
        let flutter_pointer_event = FlutterPointerEvent {
            struct_size: std::mem::size_of::<FlutterPointerEvent>(),
            phase: phase.into(),
            timestamp: self.duration_from_start().as_micros() as usize,
            x,
            y,
            device: 0,
            signal_kind: 0,
            scroll_delta_x: 0.0,
            scroll_delta_y: 0.0,
            device_kind: FlutterPointerDeviceKind_kFlutterPointerDeviceKindMouse,
            buttons: buttons.into_iter().fold(0, |acc, button| {
                acc | FlutterPointerMouseButtons::from(button) as i64
            }),
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
        }
    }
}

impl From<crossterm::event::MouseButton> for SafeMouseButton {
    fn from(value: crossterm::event::MouseButton) -> Self {
        match value {
            crossterm::event::MouseButton::Left => SafeMouseButton::Left,
            crossterm::event::MouseButton::Right => SafeMouseButton::Right,
            crossterm::event::MouseButton::Middle => SafeMouseButton::Middle,
        }
    }
}

enum SafePointerPhase {
    Up,
    Down,
}

impl From<SafePointerPhase> for FlutterPointerPhase {
    fn from(value: SafePointerPhase) -> Self {
        match value {
            SafePointerPhase::Up => FlutterPointerPhase_kUp,
            SafePointerPhase::Down => FlutterPointerPhase_kDown,
        }
    }
}

enum SafeMouseButton {
    Left,
    Right,
    Middle,
}

impl From<SafeMouseButton> for FlutterPointerMouseButtons {
    fn from(value: SafeMouseButton) -> Self {
        match value {
            SafeMouseButton::Left => FlutterPointerMouseButtons_kFlutterPointerButtonMousePrimary,
            SafeMouseButton::Right => {
                FlutterPointerMouseButtons_kFlutterPointerButtonMouseSecondary
            }
            SafeMouseButton::Middle => FlutterPointerMouseButtons_kFlutterPointerButtonMouseMiddle,
        }
    }
}
