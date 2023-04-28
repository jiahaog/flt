use crate::{semantics::FlutterSemanticsNode, sys, EmbedderCallbacks, Error, FlutterEngine, Pixel};
use std::{sync::mpsc::Sender, thread::ThreadId};

pub trait Task<T: EmbedderCallbacks> {
    fn run(&self, engine: &FlutterEngine<T>) -> Result<(), Error>;

    fn can_run_now(&self) -> bool;
}

#[derive(Debug)]
pub struct EngineTask {
    target_time_nanos: u64,
    flutter_task: sys::FlutterTask,
}

impl<T: EmbedderCallbacks> Task<T> for EngineTask {
    fn run(&self, engine: &FlutterEngine<T>) -> Result<(), Error> {
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

pub struct UserData<T: EmbedderCallbacks> {
    pub callbacks: T,
    // Remove this from user data?
    pub engine: sys::FlutterEngine,
    pub platform_thread_id: ThreadId,
    pub platform_task_channel: Sender<PlatformTask>,
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
}

#[derive(Debug)]
pub struct SemanticsUpdate {
    pub id: i32,
    pub children: Vec<i32>,
    pub node: FlutterSemanticsNode,
}

impl<T: EmbedderCallbacks> UserData<T> {
    pub fn new(
        callbacks: T,
        engine: sys::FlutterEngine,
        thread_id: ThreadId,
        platform_task_channel: Sender<PlatformTask>,
    ) -> Self {
        Self {
            callbacks,
            engine,
            platform_thread_id: thread_id,
            platform_task_channel,
        }
    }
}

pub struct TaskRunner<T: EmbedderCallbacks> {
    // This is a mutex because post_task can be called from any thread, such
    // as in the [post_task_callback].
    //
    // Interior mutability is also needed for the methods below, otherwise there
    // will be no way to get a mutable borrow to this task runner from the
    // [FlutterEngine], *and* call one of the methods that also requires a
    // mutable borrow.
    tasks: Vec<Box<dyn Task<T>>>,
}

impl<T: EmbedderCallbacks> TaskRunner<T> {
    pub fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub fn post_task(&mut self, task: impl Task<T> + 'static) {
        self.tasks.push(Box::new(task));
    }

    pub fn run_expired_tasks(&mut self, engine: &FlutterEngine<T>) -> Result<(), Error> {
        let mut not_run_tasks = vec![];
        // TODO(jiahaog): The nightly drain_filter will help here.
        // TODO(jiahaog): Or just use a priority queue.
        for task in self.tasks.drain(..) {
            if task.can_run_now() {
                task.run(engine)?;
            } else {
                not_run_tasks.push(task);
            }
        }

        for task in not_run_tasks {
            self.tasks.push(task);
        }

        Ok(())
    }
}
