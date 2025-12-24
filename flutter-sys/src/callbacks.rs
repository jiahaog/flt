use crate::{sys, EngineTask, SemanticsUpdate};

#[derive(Debug)]
pub struct PlatformMessageResponseHandle(*const sys::FlutterPlatformMessageResponseHandle);

unsafe impl Send for PlatformMessageResponseHandle {}

impl PlatformMessageResponseHandle {
    pub fn new(handle: *const sys::FlutterPlatformMessageResponseHandle) -> Self {
        Self(handle)
    }
    pub(crate) fn get(&self) -> *const sys::FlutterPlatformMessageResponseHandle {
        self.0
    }
}

#[derive(Debug)]
pub struct PlatformMessage {
    pub channel: String,
    pub message: Vec<u8>,
    pub response_handle: PlatformMessageResponseHandle,
}

pub struct Callbacks {
    pub post_platform_task_callback: Option<Box<dyn Fn(EngineTask) -> ()>>,
    pub platform_task_runs_task_on_current_thread_callback: Option<Box<dyn Fn() -> bool>>,
    pub log_message_callback: Option<Box<dyn Fn(String, String) -> ()>>,
    pub update_semantics_callback: Option<Box<dyn Fn(Vec<SemanticsUpdate>) -> ()>>,
    pub draw_callback: Option<Box<dyn Fn(&[u8], usize, usize) -> ()>>,
    pub platform_message_callback: Option<Box<dyn Fn(PlatformMessage) -> ()>>,
}
