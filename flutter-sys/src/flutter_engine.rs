use crate::embedder_callbacks::EmbedderCallbacks;
use crate::pixel::Pixel;
use crate::pointer::{FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind};
use crate::project_args::FlutterProjectArgs;
use crate::sys;
use std::slice;
use std::time::{Duration, Instant};

pub struct FlutterEngine<T: EmbedderCallbacks> {
    engine: sys::FlutterEngine,
    user_data: *mut T,
    engine_start_time: Duration,
    start_instant: Instant,
}

impl<T: EmbedderCallbacks> Drop for FlutterEngine<T> {
    fn drop(&mut self) {
        unsafe { sys::FlutterEngineShutdown(self.engine) };
        unsafe { Box::from_raw(self.user_data) };
    }
}

impl<T: EmbedderCallbacks> FlutterEngine<T> {
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

        let project_args = FlutterProjectArgs::new(assets_dir, icu_data_path);

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
        phase: FlutterPointerPhase,
        x: f64,
        y: f64,
        signal_kind: FlutterPointerSignalKind,
        scroll_delta_y: f64,
        buttons: Vec<FlutterPointerMouseButton>,
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
