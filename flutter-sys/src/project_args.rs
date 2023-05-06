use crate::{
    ffi::to_string, post_task_callback, runs_task_on_current_thread_callback,
    semantics::update_semantics_callback, sys, user_data::UserData, EngineEvent,
};
use std::ffi::CString;

pub(crate) struct FlutterProjectArgs {
    assets_path: *mut i8,
    icu_data_path: *mut i8,
    // Rust doesn't know that this needs to be in scope after being passed to
    // C.
    #[allow(unused)]
    platform_task_runner: Box<sys::FlutterTaskRunnerDescription>,
    custom_task_runners: Box<sys::FlutterCustomTaskRunners>,
}

impl Drop for FlutterProjectArgs {
    fn drop(&mut self) {
        unsafe {
            let _ = CString::from_raw(self.assets_path);
            let _ = CString::from_raw(self.icu_data_path);
        }
    }
}

impl FlutterProjectArgs {
    pub(crate) fn new(
        assets_path: &str,
        icu_data_path: &str,
        user_data: *mut std::ffi::c_void,
    ) -> Self {
        let assets_path = CString::new(assets_path).unwrap().into_raw();
        let icu_data_path = CString::new(icu_data_path).unwrap().into_raw();

        let platform_task_runner = Box::new(sys::FlutterTaskRunnerDescription {
            struct_size: std::mem::size_of::<sys::FlutterTaskRunnerDescription>(),
            user_data: user_data as *mut std::ffi::c_void,
            runs_task_on_current_thread_callback: Some(runs_task_on_current_thread_callback),
            post_task_callback: Some(post_task_callback),
            identifier: 0,
        });

        let custom_task_runners = Box::new(sys::FlutterCustomTaskRunners {
            struct_size: std::mem::size_of::<sys::FlutterCustomTaskRunners>(),
            platform_task_runner: &*platform_task_runner,
            render_task_runner: std::ptr::null(),
            thread_priority_setter: None,
        });

        Self {
            assets_path,
            icu_data_path,
            platform_task_runner,
            custom_task_runners,
        }
    }

    pub(crate) fn to_unsafe_args(&self) -> sys::FlutterProjectArgs {
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
            custom_task_runners: &*self.custom_task_runners,
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
            update_semantics_callback: Some(update_semantics_callback),
        }
    }
}

extern "C" fn log_message_callback(
    tag: *const ::std::os::raw::c_char,
    message: *const ::std::os::raw::c_char,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &UserData = unsafe { &mut *(user_data as *mut UserData) };
    let tag = to_string(tag);
    let message = to_string(message);

    user_data
        .platform_task_channel
        .send(EngineEvent::LogMessage { tag, message })
        .unwrap();
}
