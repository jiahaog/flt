use crate::event::EngineEvent;
use std::{sync::mpsc::Sender, thread::ThreadId};

pub(crate) struct UserData {
    pub platform_thread_id: ThreadId,
    pub platform_task_channel: Sender<EngineEvent>,
}

impl UserData {
    pub(crate) fn new(thread_id: ThreadId, platform_task_channel: Sender<EngineEvent>) -> Self {
        Self {
            platform_thread_id: thread_id,
            platform_task_channel,
        }
    }
}
