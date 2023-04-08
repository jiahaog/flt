#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use bmp::{px, Image, Pixel};
use clap::Parser;
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

    let num_in_row = row_bytes / 4;

    let mut img = Image::new(WIDTH as u32, HEIGHT as u32);

    // In allocation, each group of 4 bits represents a pixel. In order, each of
    // the 4 bits will be [b, g, r, a].
    for (i, v) in slice.chunks(4).enumerate() {
        let b = v[0];
        let g = v[1];
        let r = v[2];
        let _a = v[3];

        img.set_pixel(
            (i % num_in_row) as u32,
            (i / num_in_row) as u32,
            px!(r, g, b),
        );
    }
    img.save("test.bmp").unwrap();

    return true;
}

/// Simple program to greet a person
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
            log_message_callback: None,
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

        // let flutter_renderer_config = FlutterRendereConfig {};
        // let flutter_project_args = FlutterProjectArgs {};
        let mut user_data = UserData {};

        let engine_ptr: FlutterEngine = std::ptr::null_mut();

        let result = FlutterEngineRun(
            1,
            &renderer_config,
            &args.into(),
            &mut user_data as *mut UserData as *mut std::ffi::c_void,
            &engine_ptr as *const FlutterEngine as *mut FlutterEngine,
        );

        let success = result == FlutterEngineResult_kSuccess;

        println!("ran flutter success = {success}");
        let event = FlutterWindowMetricsEvent {
            struct_size: std::mem::size_of::<FlutterWindowMetricsEvent>(),
            width: WIDTH,
            height: HEIGHT,
            pixel_ratio: 1.0,
            left: 0,
            top: 0,
            physical_view_inset_top: 0.0,
            physical_view_inset_right: 0.0,
            physical_view_inset_bottom: 0.0,
            physical_view_inset_left: 0.0,
        };
        let success = FlutterEngineSendWindowMetricsEvent(
            engine_ptr,
            &event as *const FlutterWindowMetricsEvent,
        ) == FlutterEngineResult_kSuccess;
        println!("Set windowmetricevent success = {success}");

        thread::sleep(Duration::from_secs(3));
    }
}

const WIDTH: usize = 500;
const HEIGHT: usize = 400;

struct UserData {}
