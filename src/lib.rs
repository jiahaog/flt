#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use clap::Parser;
use crossterm::event::read;
use crossterm::event::MouseButton;
use crossterm::event::MouseEvent;
use std::ffi::CStr;
use std::time::Instant;
use std::{ffi::CString, slice, time::Duration};
use terminal::{Pixel, Terminal};

mod terminal;

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

    let user_data: &mut UserData = unsafe { std::mem::transmute(user_data) };
    let terminal_window = &mut user_data.terminal;

    assert_eq!(
        terminal_window.corruption_token, "foo",
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

    terminal_window.update(&buf).unwrap();

    return true;
}

extern "C" fn log_message_callback(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    _user_data: *mut ::std::os::raw::c_void,
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

pub fn main() {
    {
        let args = Args::parse();
        let renderer_config = FlutterRendererConfig {
            type_: FlutterRendererType_kSoftware,
            __bindgen_anon_1: FlutterRendererConfig__bindgen_ty_1 {
                software: FlutterSoftwareRendererConfig {
                    struct_size: std::mem::size_of::<FlutterSoftwareRendererConfig>(),
                    surface_present_callback: Some(software_surface_present_callback),
                },
            },
        };

        let (width, height) = crossterm::terminal::size().unwrap();
        let (width, height) = (width as usize, height as usize);
        // The terminal renderer merges two pixels (top and bottom) into one.
        let height = height * 2;

        let engine_ptr: FlutterEngine = std::ptr::null_mut();

        let mut terminal = Terminal::new(width, height, "foo".to_string());
        let mut user_data = UserData {
            terminal: &mut terminal,
        };

        assert_eq!(
            unsafe {
                FlutterEngineRun(
                    1,
                    &renderer_config,
                    &args.into(),
                    &mut user_data as *mut UserData as *mut std::ffi::c_void,
                    &engine_ptr as *const FlutterEngine as *mut FlutterEngine,
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
                    engine_ptr,
                    FlutterEngineDisplaysUpdateType_kFlutterEngineDisplaysUpdateTypeStartup,
                    &display as *const FlutterEngineDisplay,
                    1,
                )
            },
            FlutterEngineResult_kSuccess,
            "notify display update"
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
        assert_eq!(
            unsafe {
                FlutterEngineSendWindowMetricsEvent(
                    engine_ptr,
                    &event as *const FlutterWindowMetricsEvent,
                )
            },
            FlutterEngineResult_kSuccess,
            "Window metrics set successfully"
        );
    }

    loop {}
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

struct UserData<'a> {
    terminal: &'a mut Terminal,
}
