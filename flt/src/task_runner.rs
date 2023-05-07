use flutter_sys::{EngineTask, Error, FlutterEngine};

pub(crate) struct TaskRunner {
    tasks: Vec<EngineTask>,
}

impl TaskRunner {
    pub(crate) fn new() -> Self {
        Self { tasks: vec![] }
    }

    pub(crate) fn post_task(&mut self, task: EngineTask) {
        self.tasks.push(task);
    }

    pub(crate) fn run_expired_tasks(&mut self, engine: &FlutterEngine) -> Result<(), Error> {
        let mut not_run_tasks = vec![];
        // TODO(jiahaog): Use nightly drain_filter.
        // TODO(jiahaog): This is slow, consider a priority queue.
        for task in self.tasks.drain(..) {
            if task.can_run_now() {
                task.run(engine)?;
            } else {
                not_run_tasks.push(task);
            }
        }

        self.tasks = not_run_tasks;
        Ok(())
    }
}
