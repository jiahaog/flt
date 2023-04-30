use crate::tasks::PlatformTask;
use std::{sync::mpsc::Sender, thread::ThreadId};

pub struct UserData {
    pub platform_thread_id: ThreadId,
    pub platform_task_channel: Sender<PlatformTask>,
}

impl UserData {
    pub fn new(thread_id: ThreadId, platform_task_channel: Sender<PlatformTask>) -> Self {
        Self {
            platform_thread_id: thread_id,
            platform_task_channel,
        }
    }
}
