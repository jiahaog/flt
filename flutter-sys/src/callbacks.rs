use crate::{EngineTask, Pixel, SemanticsUpdate};

pub struct Callbacks {
    pub post_platform_task_callback: Option<Box<dyn Fn(EngineTask) -> ()>>,
    pub platform_task_runs_task_on_current_thread_callback: Option<Box<dyn Fn() -> bool>>,
    pub log_message_callback: Option<Box<dyn Fn(String, String) -> ()>>,
    pub update_semantics_callback: Option<Box<dyn Fn(Vec<SemanticsUpdate>) -> ()>>,
    pub draw_callback: Option<Box<dyn Fn(Vec<Vec<Pixel>>) -> ()>>,
}
