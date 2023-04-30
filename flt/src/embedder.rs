use crate::constants::{FPS, PIXEL_RATIO};
use crate::semantics::{draw_semantic_labels, FlutterSemanticsTree};
use crate::task_runner::TaskRunner;
use crate::terminal_event::handle_terminal_event;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{EngineEvent, FlutterEngine, FlutterTransformation};
use std::io::Write;
use std::{
    fs::File,
    sync::mpsc::{channel, Receiver},
};

pub struct TerminalEmbedder {
    engine: FlutterEngine,
    platform_task_channel: Receiver<EngineEvent>,
    semantics_tree: FlutterSemanticsTree,
    terminal_window: TerminalWindow,
    platform_task_runner: TaskRunner,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str, simple_output: bool) -> Result<Self, Error> {
        let (sender, receiver) = channel();
        let embedder = Self {
            engine: FlutterEngine::new(assets_dir, icu_data_path, sender)?,
            platform_task_channel: receiver,
            semantics_tree: FlutterSemanticsTree::new(),
            terminal_window: TerminalWindow::new(simple_output),
            platform_task_runner: TaskRunner::new(),
        };
        embedder.engine.notify_display_update(FPS as f64)?;
        embedder.engine.update_semantics(true)?;

        let (width, height) = embedder.terminal_window.size();
        embedder
            .engine
            .send_window_metrics_event(width, height, PIXEL_RATIO)?;

        Ok(embedder)
    }

    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        let mut should_run = true;

        while should_run {
            if let Ok(platform_task) = self.platform_task_channel.try_recv() {
                match platform_task {
                    EngineEvent::UpdateSemantics(updates) => {
                        self.semantics_tree.update(updates);

                        let root = self.semantics_tree.as_graph();

                        draw_semantic_labels(
                            &mut self.terminal_window,
                            FlutterTransformation::empty(),
                            root,
                        )?;

                        let mut f = File::create("/tmp/semantics.txt").unwrap();

                        writeln!(f, "{:#?}", self.semantics_tree.as_graph()).unwrap();
                    }
                    EngineEvent::Draw {
                        width,
                        height,
                        buffer,
                    } => {
                        self.terminal_window.draw(width, height, buffer)?;
                    }
                    EngineEvent::EngineTask(engine_task) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    EngineEvent::LogMessage { tag, message } => {
                        // TODO(jiahaog): Print to the main terminal.
                        println!("{tag}: {message}");
                    }
                }
            }
            should_run = handle_terminal_event(&self.engine)?;
            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }

        Ok(())
    }
}
