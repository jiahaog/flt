#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use clap::Parser;
use crossterm::terminal;
use flterminal::Pixel;
use flterminal::TerminalWindow;
use std::ffi::CStr;
use std::{ffi::CString, slice, thread, time::Duration};

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

extern "C" fn software_surface_present_callback(
    user_data: *mut std::os::raw::c_void,
    allocation: *const std::os::raw::c_void,
    row_bytes: usize,
    height: usize,
) -> bool {
    let slice: &[u8] =
        unsafe { slice::from_raw_parts(allocation as *const u8, row_bytes * height) };

    let terminal_window = user_data as *mut TerminalWindow;
    let terminal_window: &mut TerminalWindow = unsafe { std::mem::transmute(terminal_window) };

    let mut buf = vec![];

    // In allocation, each group of 4 bits represents a pixel. In order, each of
    // the 4 bits will be [b, g, r, a].
    for v in slice.chunks(4) {
        let b = v[0];
        let g = v[1];
        let r = v[2];
        let a = v[3];

        buf.push(Pixel { r, g, b, a });
    }

    terminal_window.update(&buf).unwrap();

    return true;
}

extern "C" fn log_message_callback(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    user_data: *mut ::std::os::raw::c_void,
) {
    // TODO: Print to the main terminal.
    let tag = to_string(tag);
    let message = to_string(message);
    println!("{tag}: {message}");
}

fn to_string(c_str: *const std::os::raw::c_char) -> String {
    let message = unsafe { CStr::from_ptr(c_str) };
    let message = message.to_owned();

    message.to_str().unwrap().to_string()
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    assets_dir: String,

    #[arg(long)]
    icu_data_path: String,
}

impl From<Args> for FlutterProjectArgs {
    fn from(args: Args) -> Self {
        let assets_path = args.assets_dir;
        let icu_data_path = args.icu_data_path;

        let arguments = vec!["arg".to_string()];

        FlutterProjectArgs {
            struct_size: std::mem::size_of::<FlutterProjectArgs>(),
            assets_path: CString::new(assets_path).unwrap().into_raw(),
            main_path__unused__: std::ptr::null(),
            packages_path__unused__: std::ptr::null(),
            icu_data_path: CString::new(icu_data_path).unwrap().into_raw(),
            command_line_argc: arguments.len() as i32,
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
            dart_old_gen_heap_size: -1,
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

fn main() {
    let args = Args::parse();

    unsafe {
        let renderer_config = FlutterRendererConfig {
            type_: FlutterRendererType_kSoftware,
            __bindgen_anon_1: FlutterRendererConfig__bindgen_ty_1 {
                software: FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<FlutterSoftwareRendererConfig>(),
                    surface_present_callback: Some(software_surface_present_callback),
                },
            },
        };

        let (width, height) = terminal::size().unwrap();
        let (width, height) = (width as usize, height as usize);
        // The terminal renderer merges two pixels (top and bottom) into one.
        let height = height * 2;

        let mut user_data = TerminalWindow::new(width, height);

        let engine_ptr: FlutterEngine = std::ptr::null_mut();

        let result = FlutterEngineRun(
            1,
            &renderer_config,
            &args.into(),
            &mut user_data as *mut TerminalWindow as *mut std::ffi::c_void,
            &engine_ptr as *const FlutterEngine as *mut FlutterEngine,
        );

        assert_eq!(
            result, FlutterEngineResult_kSuccess,
            "Engine started successfully"
        );

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
        let result = FlutterEngineSendWindowMetricsEvent(
            engine_ptr,
            &event as *const FlutterWindowMetricsEvent,
        );
        // println!("Set windowmetricevent success = {success}");
        assert_eq!(
            result, FlutterEngineResult_kSuccess,
            "Window metrics set successfully"
        );

        thread::sleep(Duration::from_secs(3));

        user_data.dispose();
    }
}

struct UserData {}
