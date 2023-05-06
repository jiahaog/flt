use flutter_sys::{EngineTask, Pixel, SemanticsUpdate};

/// Events that should be handled on the platform (main) thread.
#[derive(Debug)]
pub(crate) enum PlatformEvent {
    EngineEvent(EngineEvent),
    TerminalEvent(crossterm::event::Event),
}

#[derive(Debug)]
pub(crate) enum EngineEvent {
    UpdateSemantics(Vec<SemanticsUpdate>),
    Draw(Vec<Vec<Pixel>>),
    EngineTask(EngineTask),
    LogMessage { tag: String, message: String },
}
