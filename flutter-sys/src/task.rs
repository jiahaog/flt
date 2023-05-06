use crate::{sys, user_data::UserData, Error, FlutterEngine};

unsafe impl Send for EngineTask {}

#[derive(Debug)]
pub struct EngineTask {
    target_time_nanos: u64,
    flutter_task: sys::FlutterTask,
}

impl EngineTask {
    pub fn run(&self, engine: &FlutterEngine) -> Result<(), Error> {
        let result = unsafe { sys::FlutterEngineRunTask(engine.get_engine(), &self.flutter_task) };
        if result != sys::FlutterEngineResult_kSuccess {
            Err(result.into())
        } else {
            Ok(())
        }
    }

    pub fn can_run_now(&self) -> bool {
        let current_time_nanos = unsafe { sys::FlutterEngineGetCurrentTime() };
        self.target_time_nanos < current_time_nanos
    }
}

impl EngineTask {
    pub fn new(target_time_nanos: u64, flutter_task: sys::FlutterTask) -> Self {
        Self {
            target_time_nanos,
            flutter_task,
        }
    }
}

pub(crate) extern "C" fn runs_task_on_current_thread_callback(
    user_data: *mut ::std::os::raw::c_void,
) -> bool {
    let user_data: &UserData = unsafe { &mut *(user_data as *mut UserData) };

    user_data
        .callbacks
        .platform_task_runs_task_on_current_thread_callback
        .as_ref()
        .map_or(true, |callback| callback())
}

pub(crate) extern "C" fn post_platform_task_callback(
    task: sys::FlutterTask,
    target_time_nanos: u64,
    user_data: *mut ::std::os::raw::c_void,
) {
    let user_data: &UserData = unsafe { &mut *(user_data as *mut UserData) };

    let task = EngineTask::new(target_time_nanos, task);

    user_data
        .callbacks
        .post_platform_task_callback
        .as_ref()
        .map(|callback| callback(task));
}
