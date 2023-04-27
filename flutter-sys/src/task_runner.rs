use std::{cell::RefCell, collections::VecDeque, rc::Rc, thread::ThreadId};

use crate::{sys, EmbedderCallbacks, Error, FlutterEngine};

pub trait Task<T: EmbedderCallbacks> {
    fn run(&self, engine: &FlutterEngine<T>) -> Result<(), Error>;

    fn can_run_now(&self) -> bool;
}

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
    pub task_runner: TaskRunner<T>,
}

impl<T: EmbedderCallbacks> UserData<T> {
    pub fn new(
        callbacks: T,
        engine: sys::FlutterEngine,
        thread_id: ThreadId,
        task_runner: TaskRunner<T>,
    ) -> Self {
        Self {
            callbacks,
            engine,
            platform_thread_id: thread_id,
            task_runner,
        }
    }
}

pub struct TaskRunner<T: EmbedderCallbacks> {
    inner: Rc<RefCell<TaskRunnerInner<T>>>,
}

impl<T: EmbedderCallbacks> TaskRunner<T> {
    pub fn new() -> Self {
        Self {
            inner: Rc::new(RefCell::new(TaskRunnerInner::new())),
        }
    }

    pub fn post_task(&mut self, task: impl Task<T> + 'static) {
        self.inner.borrow_mut().post_task(task);
    }

    pub fn run(&self, engine: &FlutterEngine<T>) -> Result<(), Error> {
        self.inner.borrow_mut().run(engine)
    }
}

impl<T: EmbedderCallbacks> Clone for TaskRunner<T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

struct TaskRunnerInner<T: EmbedderCallbacks> {
    tasks: VecDeque<Box<dyn Task<T>>>,
}

impl<T: EmbedderCallbacks> TaskRunnerInner<T> {
    pub fn new() -> Self {
        Self {
            tasks: VecDeque::new().into(),
        }
    }

    pub fn post_task(&mut self, task: impl Task<T> + 'static) {
        println!("posttask borrow");
        let x = self.tasks.push_back(Box::new(task));
        println!("posttask release");
        x
    }

    // /// When the result is successful, returns the task when the task was not run.
    pub fn maybe_run_now(
        &mut self,
        engine: &FlutterEngine<T>,
        task: Box<dyn Task<T>>,
    ) -> Result<Option<Box<dyn Task<T>>>, Error> {
        if !task.can_run_now() {
            return Ok(Some(task));
        }

        task.run(engine)?;

        Ok(None)
    }

    pub fn run(&mut self, engine: &FlutterEngine<T>) -> Result<(), Error> {
        loop {
            {
                if let Some(task) = self.tasks.pop_front() {
                    if let Some(task) = self.maybe_run_now(engine, task)? {
                        self.tasks.push_back(task);
                    }
                };
            }
        }
    }
}
