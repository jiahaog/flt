use crate::constants::{FPS, PIXEL_RATIO};
use crate::semantics::FlutterSemanticsTree;
use crate::task_runner::TaskRunner;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{EngineEvent, FlutterEngine};
use std::io::Write;
use std::{
    fs::File,
    sync::mpsc::{channel, Receiver},
};

pub struct TerminalEmbedder {
    pub(crate) engine: FlutterEngine,
    platform_task_channel: Receiver<EngineEvent>,
    semantics_tree: FlutterSemanticsTree,
    terminal_window: TerminalWindow,
    platform_task_runner: TaskRunner,
    // TODO(jiahaog): This should be a path instead.
    debug_semantics: bool,
    show_semantics: bool,
    pub(crate) zoom: f64,
    pub(crate) mouse_down_pos: (isize, isize),
    pub(crate) prev_window_offset: (isize, isize),
    pub(crate) window_offset: (isize, isize),
    pub(crate) dimensions: (usize, usize),
}

impl TerminalEmbedder {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        simple_output: bool,
        debug_semantics: bool,
        show_semantics: bool,
    ) -> Result<Self, Error> {
        let (sender, receiver) = channel();

        let terminal_window = TerminalWindow::new(simple_output);
        let dimensions = terminal_window.size();

        let embedder = Self {
            engine: FlutterEngine::new(assets_dir, icu_data_path, sender)?,
            platform_task_channel: receiver,
            terminal_window,
            semantics_tree: FlutterSemanticsTree::new(),
            platform_task_runner: TaskRunner::new(),
            debug_semantics,
            show_semantics,
            zoom: 1.0,
            mouse_down_pos: (0, 0),
            prev_window_offset: (0, 0),
            window_offset: (0, 0),
            dimensions,
        };

        embedder.engine.notify_display_update(FPS as f64)?;
        embedder.engine.update_semantics(true)?;
        embedder.engine.send_window_metrics_event(
            embedder.dimensions.0,
            embedder.dimensions.1,
            PIXEL_RATIO,
        )?;

        Ok(embedder)
    }

    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        let mut should_run = true;

        // TODO(jiahaog): Don't spin.
        while should_run {
            if let Ok(platform_task) = self.platform_task_channel.try_recv() {
                match platform_task {
                    EngineEvent::UpdateSemantics(updates) => {
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
                    EngineEvent::Draw(pixel_grid) => {
                        self.terminal_window.draw(pixel_grid, self.window_offset)?;
                    }
                    EngineEvent::EngineTask(engine_task) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    EngineEvent::LogMessage { tag, message } => {
                        // TODO(jiahaog): Print to the main terminal.
                        println!("{tag}: {message}");
                    }
                };
            }

            if let Ok(terminal_event) = self.terminal_window.event_channel().try_recv() {
                should_run = self.handle_terminal_event(terminal_event)?;
            }

            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }

        Ok(())
    }
}
