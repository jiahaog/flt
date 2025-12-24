use crate::{Error, TerminalEmbedder};
use flutter_sys::{EngineTask, SemanticsUpdate};
use std::fs::File;
use std::io::Write;

/// Events that should be handled on the platform (main) thread.
#[derive(Debug)]
pub(crate) enum PlatformEvent {
    EngineEvent(EngineEvent),
    TerminalEvent(crossterm::event::Event),
}

#[derive(Debug)]
pub(crate) enum EngineEvent {
    UpdateSemantics(Vec<SemanticsUpdate>),
    Draw(Vec<u8>, usize, usize),
    EngineTask(EngineTask),
    LogMessage { tag: String, message: String },
    PlatformMessage(flutter_sys::PlatformMessage),
}

impl TerminalEmbedder {
    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        // TODO(jiahaog): Consider async Rust or Tokio instead.
        // TODO(jiahaog): It is a mistake to handle input events and drawing on the same thread.
        while self.should_run {
            if let Ok(platform_task) = self.platform_events.recv() {
                match platform_task {
                    PlatformEvent::EngineEvent(EngineEvent::UpdateSemantics(updates)) => {
                        self.semantics_tree.update(updates);

                        self.terminal_window
                            .update_semantics(self.semantics_tree.as_label_positions());

                        if self.debug_semantics {
                            let mut f = File::create("/tmp/flt-semantics.txt").unwrap();
                            writeln!(f, "{:#?}", self.semantics_tree.as_graph()).unwrap();
                        }
                    }
                    PlatformEvent::EngineEvent(EngineEvent::Draw(buffer, width, height)) => {
                        // Not sure if doing this on every frame is ok, hoping that the engine has
                        // some mechanism to make this a no-op if the parameters are unchanged.
                        self.engine.send_window_metrics_event(
                            (
                                (self.dimensions.0 as f64 * self.zoom).round() as usize,
                                (self.dimensions.1 as f64 * self.zoom).round() as usize,
                            ),
                            self.terminal_window.device_pixel_ratio() * self.zoom * self.scale,
                        )?;

                        self.terminal_window
                            .draw(buffer, width, height, self.window_offset)?;
                    }
                    PlatformEvent::EngineEvent(EngineEvent::EngineTask(engine_task)) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    PlatformEvent::EngineEvent(EngineEvent::LogMessage { tag, message }) => {
                        // TODO(jiahaog): Print to the main terminal.
                        self.terminal_window.log(format!("{tag}: {message}"));
                    }
                    PlatformEvent::EngineEvent(EngineEvent::PlatformMessage(message)) => {
                        if !flutter_sys::text_input::handle_message(&message) {
                            self.engine.send_platform_message_response(
                                message.response_handle,
                                None,
                            )?;
                        }
                    }
                    PlatformEvent::TerminalEvent(event) => {
                        self.handle_terminal_event(event)?;
                    }
                };
            }

            // TODO(jiahaog): Doing it like this probably makes us only able to run expired
            // tasks when a platform event is received.
            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }

        Ok(())
    }
}
