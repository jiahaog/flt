use crate::embedder_callbacks::EmbedderCallbacks;
use crate::pixel::Pixel;
use crate::pointer::{FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind};
use crate::project_args::FlutterProjectArgs;
use crate::{sys, KeyEventType};
use std::slice;
use std::time::{Duration, Instant};

pub struct FlutterEngine<T: EmbedderCallbacks> {
    // `UserData` needs to be on the heap so that the Flutter Engine
    // callbacks can safely provide a pointer to it (if it was on the
    // stack, there is a chance that the value is dropped when the
    // callbacks still reference it).
    user_data: Box<UserData<T>>,
    engine_start_time: Duration,
    start_instant: Instant,
}

pub struct UserData<T: EmbedderCallbacks> {
    pub callbacks: T,
    pub engine: sys::FlutterEngine,
}

impl<T: EmbedderCallbacks> Drop for FlutterEngine<T> {
    fn drop(&mut self) {
        unsafe { sys::FlutterEngineShutdown(self.get_engine()) };
    }
}

impl<T: EmbedderCallbacks> FlutterEngine<T> {
    pub fn new(assets_dir: &str, icu_data_path: &str, callbacks: T) -> Result<Self, Error> {
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

        let mut user_data = Box::new(UserData {
            callbacks,
            engine: std::ptr::null_mut(),
        });

        let user_data_ptr: *mut UserData<T> = &mut *user_data;
        let user_data_ptr: *mut std::ffi::c_void = user_data_ptr as *mut std::ffi::c_void;

        let mut engine_ptr: sys::FlutterEngine = std::ptr::null_mut();

        let result = unsafe {
            sys::FlutterEngineRun(
                1,
                &renderer_config,
                &project_args.to_unsafe_args::<T>(user_data_ptr) as *const sys::FlutterProjectArgs,
                user_data_ptr,
                &mut engine_ptr,
            )
        };

        user_data.engine = engine_ptr;

        let embedder = Self {
            user_data,

            engine_start_time: Duration::from_nanos(unsafe { sys::FlutterEngineGetCurrentTime() }),
            start_instant: Instant::now(),
        };
        match result {
            sys::FlutterEngineResult_kSuccess => Ok(embedder),
            err => Err(err.into()),
        }
    }

    /// Returns a duration from when the Flutter Engine was started.
    fn duration_from_start(&self) -> Duration {
        // Always offset instants from `engine_start_time` to match the engine time base.
        Instant::now().duration_since(self.start_instant) + self.engine_start_time
    }

    fn get_engine(&self) -> sys::FlutterEngine {
        self.user_data.engine
    }

    pub fn update_semantics(&self, enabled: bool) -> Result<(), Error> {
        let result =
            unsafe { sys::FlutterEngineUpdateSemanticsEnabled(self.get_engine(), enabled) };
        match result {
            sys::FlutterEngineResult_kSuccess => Ok(()),
            err => Err(err.into()),
        }
    }

    pub fn notify_display_update(&self, refresh_rate: f64) -> Result<(), Error> {
        let display = sys::FlutterEngineDisplay {
            struct_size: std::mem::size_of::<sys::FlutterEngineDisplay>(),
            display_id: 0,
            single_display: true,
            refresh_rate,
        };

        let result = unsafe {
            sys::FlutterEngineNotifyDisplayUpdate(
                self.get_engine(),
                sys::FlutterEngineDisplaysUpdateType_kFlutterEngineDisplaysUpdateTypeStartup,
                &display as *const sys::FlutterEngineDisplay,
                1,
            )
        };
        match result {
            sys::FlutterEngineResult_kSuccess => Ok(()),
            err => Err(err.into()),
        }
    }

    pub fn send_window_metrics_event(
        &self,
        width: usize,
        height: usize,
        pixel_ratio: f64,
    ) -> Result<(), Error> {
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

        let result = unsafe {
            sys::FlutterEngineSendWindowMetricsEvent(
                self.get_engine(),
                &event as *const sys::FlutterWindowMetricsEvent,
            )
        };
        match result {
            sys::FlutterEngineResult_kSuccess => Ok(()),
            err => Err(err.into()),
        }
    }

    pub fn send_pointer_event(
        &self,
        phase: FlutterPointerPhase,
        x: f64,
        y: f64,
        signal_kind: FlutterPointerSignalKind,
        scroll_delta_y: f64,
        buttons: Vec<FlutterPointerMouseButton>,
    ) -> Result<(), Error> {
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

        let result = unsafe {
            sys::FlutterEngineSendPointerEvent(self.get_engine(), &flutter_pointer_event, 1)
        };
        match result {
            sys::FlutterEngineResult_kSuccess => Ok(()),
            err => Err(err.into()),
        }
    }

    // TODO(jiahaog): Actually implement this.
    #[allow(unused)]
    pub fn send_key_event(&self, event_type: KeyEventType, c: char) -> Result<(), Error> {
        // let assets_path = CString::new("abc").unwrap().into_raw();

        // let flutter_key_event = sys::FlutterKeyEvent {
        //     struct_size: std::mem::size_of::<sys::FlutterKeyEvent>(),
        //     timestamp: self.duration_from_start().as_micros() as f64,
        //     type_: event_type.into(),
        //     // KeyI
        //     physical: 0x0007000c,
        //     logical: 0x00000000069,
        //     character: assets_path,
        //     synthesized: false,
        // };

        // let result = unsafe {
        //     sys::FlutterEngineSendKeyEvent(self.engine, &flutter_key_event, None, null_mut())
        // };
        // match result {
        //     sys::FlutterEngineResult_kSuccess => Ok(()),
        //     err => Err(err.into()),
        // }
        Ok(())
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

    let user_data: &mut UserData<T> = unsafe { std::mem::transmute(user_data) };

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

    user_data.callbacks.draw(width, height, buf);

    return true;
}

#[derive(Debug)]
pub enum Error {
    InvalidLibraryVersion,
    InvalidArguments,
    InternalConsistency,
}

impl From<sys::FlutterEngineResult> for Error {
    fn from(value: sys::FlutterEngineResult) -> Self {
        match value {
            sys::FlutterEngineResult_kInvalidLibraryVersion => Error::InvalidLibraryVersion,
            sys::FlutterEngineResult_kInvalidArguments => Error::InvalidArguments,
            sys::FlutterEngineResult_kInternalInconsistency => Error::InternalConsistency,
            value => panic!("Unexpected value for FlutterEngineResult: {} ", value),
        }
    }
}
