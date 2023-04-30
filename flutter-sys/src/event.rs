use crate::{EngineTask, FlutterSemanticsNode, Pixel};

#[derive(Debug)]
pub enum EngineEvent {
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

#[derive(Debug)]
pub struct SemanticsUpdate {
    pub id: i32,
    pub children: Vec<i32>,
    pub node: FlutterSemanticsNode,
}
