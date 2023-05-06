use crate::event::EngineEvent;
use crate::pixel::Pixel;
use crate::pointer::{FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind};
use crate::project_args::FlutterProjectArgs;
use crate::user_data::UserData;
use crate::{sys, Error, KeyEventType};
use std::slice;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

pub struct FlutterEngine {
    engine: sys::FlutterEngine,
    // `UserData` needs to be on the heap so that the Flutter Engine
    // callbacks can safely provide a pointer to it (if it was on the
    // stack, there is a chance that the value is dropped when the
    // callbacks still reference it).
    //
    // The unused exemption is because Rust doesn't know that it is used in the
    // C callbacks.
    #[allow(unused)]
    user_data: Box<UserData>,
    // TODO(jiahaog): Remove this and introduce a clock instead.
    engine_start_time: Duration,
    start_instant: Instant,
}

impl Drop for FlutterEngine {
    fn drop(&mut self) {
        unsafe { sys::FlutterEngineShutdown(self.get_engine()) };
    }
}

impl FlutterEngine {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        platform_task_channel: Sender<EngineEvent>,
    ) -> Result<Self, Error> {
        let renderer_config = sys::FlutterRendererConfig {
            type_: sys::FlutterRendererType_kSoftware,
            __bindgen_anon_1: sys::FlutterRendererConfig__bindgen_ty_1 {
                software: sys::FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<sys::FlutterSoftwareRendererConfig>(),
                    surface_present_callback: Some(software_surface_present_callback),
                },
            },
        };

        let mut user_data = Box::new(UserData::new(
            std::thread::current().id(),
            platform_task_channel,
        ));

        let user_data_ptr: *mut UserData = &mut *user_data;
        let user_data_ptr: *mut std::ffi::c_void = user_data_ptr as *mut std::ffi::c_void;

        let project_args = FlutterProjectArgs::new(assets_dir, icu_data_path, user_data_ptr);

        let mut engine = Self {
            engine: std::ptr::null_mut(),
            user_data,
            engine_start_time: Duration::from_nanos(unsafe { sys::FlutterEngineGetCurrentTime() }),
            start_instant: Instant::now(),
        };

        let result = unsafe {
            sys::FlutterEngineRun(
                1,
                &renderer_config,
                &project_args.to_unsafe_args() as *const sys::FlutterProjectArgs,
                user_data_ptr,
                &mut engine.engine,
            )
        };
        if result != sys::FlutterEngineResult_kSuccess {
            return Err(result.into());
        };

        Ok(engine)
    }

    /// Returns a duration from when the Flutter Engine was started.
    fn duration_from_start(&self) -> Duration {
        // Always offset instants from `engine_start_time` to match the engine time
        // base.
        Instant::now().duration_since(self.start_instant) + self.engine_start_time
    }

    pub(crate) fn get_engine(&self) -> sys::FlutterEngine {
        self.engine
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
        (width, height): (usize, usize),
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
        (x, y): (f64, f64),
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
        //     sys::FlutterEngineSendKeyEvent(self.engine, &flutter_key_event, None,
        // null_mut()) };
        // match result {
        //     sys::FlutterEngineResult_kSuccess => Ok(()),
        //     err => Err(err.into()),
        // }
        Ok(())
    }
}

extern "C" fn software_surface_present_callback(
    user_data: *mut std::os::raw::c_void,
    allocation: *const std::os::raw::c_void,
    row_bytes: usize,
    height: usize,
) -> bool {
    let allocation: &[u8] =
        unsafe { slice::from_raw_parts(allocation as *const u8, row_bytes * height) };

    let user_data: &mut UserData = unsafe { std::mem::transmute(user_data) };

    // In allocation, each group of 4 bits represents a pixel. In order, each of
    // the 4 bits will be [b, g, r, a].
    let buffer = allocation
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

    let pixel_grid = buffer
        .into_iter()
        .enumerate()
        .fold(vec![], |mut acc, (i, pixel)| {
            let x = i % width;
            if x == 0 {
                acc.push(vec![]);
            }

            acc.last_mut().unwrap().push(pixel);
            acc
        });

    user_data
        .platform_task_channel
        .send(EngineEvent::Draw(pixel_grid))
        .unwrap();

    return true;
}
