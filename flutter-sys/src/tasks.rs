use crate::{sys, Error, FlutterEngine, Pixel, SemanticsUpdate};

pub trait Task {
    fn run(&self, engine: &FlutterEngine) -> Result<(), Error>;

    fn can_run_now(&self) -> bool;
}

#[derive(Debug)]
pub struct EngineTask {
    target_time_nanos: u64,
    flutter_task: sys::FlutterTask,
}

impl Task for EngineTask {
    fn run(&self, engine: &FlutterEngine) -> Result<(), Error> {
        let result = unsafe { sys::FlutterEngineRunTask(engine.get_engine(), &self.flutter_task) };
        if result != sys::FlutterEngineResult_kSuccess {
            Err(result.into())
        } else {
            Ok(())
        }
    }

    fn can_run_now(&self) -> bool {
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

#[derive(Debug)]
pub enum PlatformTask {
    UpdateSemantics(Vec<SemanticsUpdate>),
    Draw {
        width: usize,
        height: usize,
        buffer: Vec<Pixel>,
    },
    EngineTask(EngineTask),
    LogMessage {
        tag: String,
        message: String,
    },
}
