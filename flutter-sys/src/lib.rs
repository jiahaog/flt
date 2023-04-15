mod sys {
    #![allow(unused)]
    #![allow(non_upper_case_globals)]
    #![allow(non_camel_case_types)]
    #![allow(non_snake_case)]
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi::{CStr, CString};
use std::slice;
use std::time::{Duration, Instant};

use sys::{FlutterPointerPhase_kHover, FlutterPointerSignalKind_kFlutterPointerSignalKindScroll};

extern "C" fn software_surface_present_callback<T: EmbedderCallbacks>(
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
    fn to_unsafe_args<T: EmbedderCallbacks>(&self) -> sys::FlutterProjectArgs {
        sys::FlutterProjectArgs {
            struct_size: std::mem::size_of::<sys::FlutterProjectArgs>(),
            assets_path: self.assets_path,
            main_path__unused__: std::ptr::null(),
            packages_path__unused__: std::ptr::null(),
            icu_data_path: self.icu_data_path,
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
            log_message_callback: Some(log_message_callback::<T>),
            log_tag: std::ptr::null(),
            on_pre_engine_restart_callback: None,
            update_semantics_callback: None,
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

extern "C" fn log_message_callback<T: EmbedderCallbacks>(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &mut T = unsafe { std::mem::transmute(user_data) };
    let tag = to_string(tag);
    let message = to_string(message);

    user_data.log(tag, message);
}

fn to_string(c_str: *const std::os::raw::c_char) -> String {
    let message = unsafe { CStr::from_ptr(c_str) };
    let message = message.to_owned();

    message.to_str().unwrap().to_string()
}

pub trait EmbedderCallbacks {
    fn log(&self, tag: String, message: String);

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>);
}

pub struct SafeEngine<T: EmbedderCallbacks> {
    engine: sys::FlutterEngine,
    user_data: *mut T,
    engine_start_time: Duration,
    start_instant: Instant,
}

impl<T: EmbedderCallbacks> Drop for SafeEngine<T> {
    fn drop(&mut self) {
        unsafe { sys::FlutterEngineShutdown(self.engine) };
        unsafe { Box::from_raw(self.user_data) };
    }
}

impl<T: EmbedderCallbacks> SafeEngine<T> {
    pub fn new(assets_dir: &str, icu_data_path: &str, callbacks: T) -> Self {
        let renderer_config = sys::FlutterRendererConfig {
            type_: sys::FlutterRendererType_kSoftware,
            __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                software: sys::FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<sys::FlutterSoftwareRendererConfig>(),
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
            user_data: Box::into_raw(callbacks.into()),

            engine_start_time: Duration::from_nanos(unsafe { sys::FlutterEngineGetCurrentTime() }),
            start_instant: Instant::now(),
        };

        let user_data_ptr = embedder.user_data;

        assert_eq!(
            unsafe {
                sys::FlutterEngineRun(
                    1,
                    &renderer_config,
                    &project_args.to_unsafe_args::<T>() as *const sys::FlutterProjectArgs,
                    user_data_ptr as *mut std::ffi::c_void,
                    &embedder.engine as *const sys::FlutterEngine as *mut sys::FlutterEngine,
                )
            },
            sys::FlutterEngineResult_kSuccess,
            "Engine started successfully"
        );

        embedder
    }

    /// Returns a duration from when the Flutter Engine was started.
    pub fn duration_from_start(&self) -> Duration {
        // Always offset instants from `engine_start_time` to match the engine time base.
        Instant::now().duration_since(self.start_instant) + self.engine_start_time
    }

    pub fn notify_display_update(&self, refresh_rate: f64) {
        let display = sys::FlutterEngineDisplay {
            struct_size: std::mem::size_of::<sys::FlutterEngineDisplay>(),
            display_id: 0,
            single_display: true,
            refresh_rate,
        };

        assert_eq!(
            unsafe {
                sys::FlutterEngineNotifyDisplayUpdate(
                    self.engine,
                    sys::FlutterEngineDisplaysUpdateType_kFlutterEngineDisplaysUpdateTypeStartup,
                    &display as *const sys::FlutterEngineDisplay,
                    1,
                )
            },
            sys::FlutterEngineResult_kSuccess,
            "notify display update"
        );
    }

    pub fn send_window_metrics_event(&self, width: usize, height: usize, pixel_ratio: f64) {
        let event = sys::FlutterWindowMetricsEvent {
            struct_size: std::mem::size_of::<sys::FlutterWindowMetricsEvent>(),
            width,
            height,
            pixel_ratio,
            left: 0,
            top: 0,
            physical_view_inset_top: 0.0,
            physical_view_inset_right: 0.0,
            physical_view_inset_bottom: 0.0,
            physical_view_inset_left: 0.0,
        };
        assert_eq!(
            unsafe {
                sys::FlutterEngineSendWindowMetricsEvent(
                    self.engine,
                    &event as *const sys::FlutterWindowMetricsEvent,
                )
            },
            sys::FlutterEngineResult_kSuccess,
            "Window metrics set successfully"
        );
    }

    pub fn send_pointer_event(
        &self,
        phase: SafePointerPhase,
        x: f64,
        y: f64,
        signal_kind: SafeSignalKind,
        scroll_delta_y: f64,
        buttons: Vec<SafeMouseButton>,
    ) {
        let flutter_pointer_event = sys::FlutterPointerEvent {
            struct_size: std::mem::size_of::<sys::FlutterPointerEvent>(),
            phase: phase.into(),
            // phase: FlutterPointerPhase_kHover,
            timestamp: self.duration_from_start().as_micros() as usize,
            x,
            y,
            device: 0,
            signal_kind: signal_kind.into(),
            scroll_delta_x: 0.0,
            scroll_delta_y,
            device_kind: sys::FlutterPointerDeviceKind_kFlutterPointerDeviceKindMouse,
            buttons: buttons.into_iter().fold(0, |acc, button| {
                acc | sys::FlutterPointerMouseButtons::from(button) as i64
            }),
            // buttons: sys::FlutterPointerMouseButtons_kFlutterPointerButtonMouseMiddle as i64,
            pan_x: 0.0,
            pan_y: 0.0,
            scale: 0.0,
            rotation: 0.0,
        };

        unsafe {
            assert_eq!(
                sys::FlutterEngineSendPointerEvent(self.engine, &flutter_pointer_event, 1),
                sys::FlutterEngineResult_kSuccess
            );
        }
    }
}

pub enum SafePointerPhase {
    Up,
    Down,
    Hover,
}

pub enum SafeSignalKind {
    None,
    Scroll,
}

impl From<SafeSignalKind> for sys::FlutterPointerSignalKind {
    fn from(value: SafeSignalKind) -> Self {
        match value {
            SafeSignalKind::None => sys::FlutterPointerSignalKind_kFlutterPointerSignalKindNone,
            SafeSignalKind::Scroll => sys::FlutterPointerSignalKind_kFlutterPointerSignalKindScroll,
        }
    }
}

impl From<SafePointerPhase> for sys::FlutterPointerPhase {
    fn from(value: SafePointerPhase) -> Self {
        match value {
            SafePointerPhase::Up => sys::FlutterPointerPhase_kUp,
            SafePointerPhase::Down => sys::FlutterPointerPhase_kDown,
            SafePointerPhase::Hover => sys::FlutterPointerPhase_kHover,
        }
    }
}

pub enum SafeMouseButton {
    Left,
    Right,
    Middle,
}

impl From<SafeMouseButton> for sys::FlutterPointerMouseButtons {
    fn from(value: SafeMouseButton) -> Self {
        match value {
            SafeMouseButton::Left => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMousePrimary
            }
            SafeMouseButton::Right => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMouseSecondary
            }
            SafeMouseButton::Middle => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMouseMiddle
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Pixel {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Pixel {
    pub fn zero() -> Self {
        Pixel {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        }
    }
}
