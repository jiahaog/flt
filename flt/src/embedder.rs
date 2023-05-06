use crate::constants::{FPS, PIXEL_RATIO};
use crate::semantics::FlutterSemanticsTree;
use crate::task_runner::TaskRunner;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{EngineEvent, FlutterEngine};
use std::io::Write;
use std::thread;
use std::{
    fs::File,
    sync::mpsc::{channel, Receiver},
};

pub struct TerminalEmbedder {
    pub(crate) engine: FlutterEngine,
    platform_events: Receiver<PlatformEvent>,
    semantics_tree: FlutterSemanticsTree,
    pub(crate) terminal_window: TerminalWindow,
    platform_task_runner: TaskRunner,
    // TODO(jiahaog): This should be a path instead.
    debug_semantics: bool,
    pub(crate) show_semantics: bool,
    pub(crate) zoom: f64,
    pub(crate) mouse_down_pos: (isize, isize),
    pub(crate) prev_window_offset: (isize, isize),
    pub(crate) window_offset: (isize, isize),
    pub(crate) dimensions: (usize, usize),
}

enum PlatformEvent {
    EngineEvent(EngineEvent),
    TerminalEvent(crossterm::event::Event),
}

impl TerminalEmbedder {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        simple_output: bool,
        debug_semantics: bool,
    ) -> Result<Self, Error> {
        let (main_sender, main_receiver) = channel();
        let (engine_sender, engine_receiver) = channel();
        let (terminal_sender, terminal_receiver) = channel();

        let terminal_window = TerminalWindow::new(simple_output, terminal_sender);

        // Compose channels into one because there is no other way to do a blocking
        // subscription to multiple channels simutaneously without dependencies.
        // TODO(jiahaog): Consider async Rust or Tokio instead.
        {
            let engine_main_sender = main_sender.clone();
            thread::spawn(move || {
                for event in engine_receiver {
                    engine_main_sender
                        .send(PlatformEvent::EngineEvent(event))
                        .unwrap();
                }
            });

            let terminal_main_sender = main_sender.clone();
            thread::spawn(move || {
                for event in terminal_receiver {
                    terminal_main_sender
                        .send(PlatformEvent::TerminalEvent(event))
                        .unwrap();
                }
            });
        }

        let dimensions = terminal_window.size();

        let embedder = Self {
            engine: FlutterEngine::new(assets_dir, icu_data_path, engine_sender)?,
            platform_events: main_receiver,
            terminal_window,
            semantics_tree: FlutterSemanticsTree::new(),
            platform_task_runner: TaskRunner::new(),
            debug_semantics,
            show_semantics: false,
            zoom: 1.0,
            mouse_down_pos: (0, 0),
            prev_window_offset: (0, 0),
            window_offset: (0, 0),
            dimensions,
        };

        embedder.engine.notify_display_update(FPS as f64)?;
        embedder
            .engine
            .send_window_metrics_event(embedder.dimensions, PIXEL_RATIO)?;

        Ok(embedder)
    }

    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        let mut should_run = true;

        while should_run {
            if let Ok(platform_task) = self.platform_events.recv() {
                match platform_task {
                    PlatformEvent::EngineEvent(EngineEvent::UpdateSemantics(updates)) => {
                        self.semantics_tree.update(updates);

                        if self.show_semantics {
                            self.terminal_window
                                .update_semantics(self.semantics_tree.as_label_positions());
                        }

                        if self.debug_semantics {
                            let mut f = File::create("/tmp/flt-semantics.txt").unwrap();
                            writeln!(f, "{:#?}", self.semantics_tree.as_graph()).unwrap();
                        }
                    }
                    PlatformEvent::EngineEvent(EngineEvent::Draw(pixel_grid)) => {
                        self.terminal_window.draw(pixel_grid, self.window_offset)?;
                    }
                    PlatformEvent::EngineEvent(EngineEvent::EngineTask(engine_task)) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    PlatformEvent::EngineEvent(EngineEvent::LogMessage { tag, message }) => {
                        // TODO(jiahaog): Print to the main terminal.
                        println!("{tag}: {message}");
                    }
                    PlatformEvent::TerminalEvent(event) => {
                        should_run = self.handle_terminal_event(event)?;
                    }
                };
            }

            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }

        Ok(())
    }
}
