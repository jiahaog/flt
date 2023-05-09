use crate::{constants::DEFAULT_PIXEL_RATIO, Error, TerminalEmbedder};
use flutter_sys::{EngineTask, Pixel, SemanticsUpdate};
use std::fs::File;
use std::io::Write;
use std::time::Instant;

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

impl TerminalEmbedder {
    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        // TODO(jiahaog): Consider async Rust or Tokio instead.
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
                    PlatformEvent::EngineEvent(EngineEvent::Draw(pixel_grid)) => {
                        let current_frame_instant = Instant::now();
                        let prev_frame_duration =
                            current_frame_instant.duration_since(self.last_frame_instant);
                        self.last_frame_instant = current_frame_instant;
                        // Not sure if doing this on every frame is ok, hoping that the engine has
                        // some mechanism to make this a no-op if the parameters are unchanged.
                        self.engine.send_window_metrics_event(
                            (
                                (self.dimensions.0 as f64 * self.zoom).round() as usize,
                                (self.dimensions.1 as f64 * self.zoom).round() as usize,
                            ),
                            DEFAULT_PIXEL_RATIO * self.zoom * self.scale,
                        )?;

                        self.terminal_window.draw(
                            pixel_grid,
                            self.window_offset,
                            prev_frame_duration,
                        )?;
                    }
                    PlatformEvent::EngineEvent(EngineEvent::EngineTask(engine_task)) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    PlatformEvent::EngineEvent(EngineEvent::LogMessage { tag, message }) => {
                        // TODO(jiahaog): Print to the main terminal.
                        self.terminal_window.log(format!("{tag}: {message}"));
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
