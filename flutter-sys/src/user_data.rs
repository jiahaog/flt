use crate::{sys, tasks::PlatformTask};
use std::{sync::mpsc::Sender, thread::ThreadId};

pub struct UserData {
    // TODO(jiahaog): Remove this from user data?
    pub engine: sys::FlutterEngine,
    pub platform_thread_id: ThreadId,
    pub platform_task_channel: Sender<PlatformTask>,
}

impl UserData {
    pub fn new(
        engine: sys::FlutterEngine,
        thread_id: ThreadId,
        platform_task_channel: Sender<PlatformTask>,
    ) -> Self {
        Self {
            engine,
            platform_thread_id: thread_id,
            platform_task_channel,
        }
    }
}
