use crate::{embedder_callbacks::EmbedderCallbacks, sys};
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
};

pub struct FlutterProjectArgs {
    assets_path: *mut i8,
    icu_data_path: *mut i8,
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
    pub fn new(assets_path: &str, icu_data_path: &str) -> Self {
        let assets_path = CString::new(assets_path).unwrap().into_raw();
        let icu_data_path = CString::new(icu_data_path).unwrap().into_raw();

        Self {
            assets_path,
            icu_data_path,
        }
    }

    pub fn to_unsafe_args<T: EmbedderCallbacks>(&self) -> sys::FlutterProjectArgs {
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
            update_semantics_callback: Some(update_semantics_callback),
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

extern "C" fn update_semantics_callback(
    semantics_update: *const sys::FlutterSemanticsUpdate,
    user_data: *mut ::std::os::raw::c_void,
) {
    println!("update semantics callback");
    // let sys::FlutterSemanticsUpdate {
    //     nodes_count, nodes, ..
    // } = unsafe { *semantics_update };

    // let nodes = unsafe { std::slice::from_raw_parts(nodes, nodes_count) };

    // let tree = FlutterSemanticsTree::from_nodes(nodes);
    // println!("root: {:?}", tree.root());
}

#[derive(Debug)]
struct FlutterSemanticsTree<'a> {
    map: HashMap<i32, &'a sys::FlutterSemanticsNode>,
}

impl<'a> FlutterSemanticsTree<'a> {
    fn from_nodes(nodes: &'a [sys::FlutterSemanticsNode]) -> Self {
        Self {
            map: nodes.into_iter().map(|node| (node.id, node)).collect(),
        }
    }

    fn root(&self) -> FlutterSemanticsNode {
        let root_id = 0;

        self.root_recur(root_id)
    }

    fn root_recur(&self, id: i32) -> FlutterSemanticsNode {
        let &&sys::FlutterSemanticsNode {
            children_in_traversal_order,
            child_count,
            label,
            ..
        } = self.map.get(&id).expect("Node ID must always be present");

        let children_ids =
            unsafe { std::slice::from_raw_parts(children_in_traversal_order, child_count) };

        let children = children_ids
            .into_iter()
            .map(|id| self.root_recur(*id))
            .collect();

        FlutterSemanticsNode {
            children,
            label: to_string(label),
        }
    }
}

#[derive(Debug)]
pub struct FlutterSemanticsNode {
    children: Vec<FlutterSemanticsNode>,
    label: String,
}
