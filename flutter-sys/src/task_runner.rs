use std::{collections::VecDeque, thread::ThreadId};

use crate::{sys, EmbedderCallbacks, Error};

pub struct Task {
    target_time_nanos: u64,
    flutter_task: sys::FlutterTask,
}

impl Task {
    pub fn new(target_time_nanos: u64, flutter_task: sys::FlutterTask) -> Self {
        Self {
            target_time_nanos,
            flutter_task,
        }
    }

    fn can_run_now(&self) -> bool {
        let current_time_nanos = unsafe { sys::FlutterEngineGetCurrentTime() };
        self.target_time_nanos < current_time_nanos
    }
}

pub struct UserData<T: EmbedderCallbacks> {
    pub callbacks: T,
    pub engine: sys::FlutterEngine,
    pub platform_thread_id: ThreadId,
    tasks: VecDeque<Task>,
}

impl<T: EmbedderCallbacks> Drop for UserData<T> {
    fn drop(&mut self) {
        println!("droppign user data");
    }
}

impl<T: EmbedderCallbacks> UserData<T> {
    pub fn new(callbacks: T, engine: sys::FlutterEngine, thread_id: ThreadId) -> Self {
        println!("creating user data");
        Self {
            callbacks,
            engine,
            platform_thread_id: thread_id,

            tasks: VecDeque::new(),
        }
    }

    pub fn post_task(&mut self, task: Task) {
        self.tasks.push_back(task)
    }

    /// When the result is successful, returns the task when the task was not run.
    pub fn maybe_run_now(&mut self, task: Task) -> Result<Option<Task>, Error> {
        if !task.can_run_now() {
            return Ok(Some(task));
        }

        let result = unsafe { sys::FlutterEngineRunTask(self.engine, &task.flutter_task) };
        if result != sys::FlutterEngineResult_kSuccess {
            return Err(result.into());
        }

        Ok(None)
    }

    pub fn run<F: Fn() -> Result<(), Error>>(&mut self, callback: F) -> Result<(), Error> {
        loop {
            if let Some(task) = self.tasks.pop_front() {
                if let Some(task) = self.maybe_run_now(task)? {
                    self.tasks.push_back(task);
                }
            };
            callback()?;
        }
    }
}
