use constants::{FPS, PIXEL_RATIO};
use flutter_sys::{
    FlutterEngine, FlutterSemanticsTree, FlutterTransformation, GraphNode, PlatformTask,
};
use std::io::Write;
use std::{
    fs::File,
    sync::mpsc::{channel, Receiver},
};
use task_runner::TaskRunner;
use terminal_event_task::TerminalEventTask;
use terminal_window::TerminalWindow;

mod constants;
mod task_runner;
mod terminal_event_task;
mod terminal_window;

pub struct TerminalEmbedder {
    engine: FlutterEngine,
    platform_task_channel: Receiver<PlatformTask>,
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
        loop {
            if let Ok(platform_task) = self.platform_task_channel.try_recv() {
                match platform_task {
                    PlatformTask::UpdateSemantics(updates) => {
                        self.semantics_tree.update(updates);

                        let root = self.semantics_tree.as_graph();

                        draw_semantic_labels(
                            &mut self.terminal_window,
                            FlutterTransformation::empty(),
                            root,
                        )?;

                        let mut f = File::create("/tmp/semantics.txt").unwrap();

                        writeln!(f, "{:#?}", self.semantics_tree).unwrap();
                    }
                    PlatformTask::Draw {
                        width,
                        height,
                        buffer,
                    } => {
                        self.terminal_window.draw(width, height, buffer)?;
                    }
                    PlatformTask::EngineTask(engine_task) => {
                        self.platform_task_runner.post_task(engine_task);
                    }
                    PlatformTask::LogMessage { tag, message } => {
                        // TODO(jiahaog): Print to the main terminal.
                        println!("{tag}: {message}");
                    }
                }
            }
            self.platform_task_runner.post_task(TerminalEventTask {});
            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }
    }
}

#[derive(Debug)]
pub enum Error {
    EngineError(flutter_sys::Error),
    TerminalError(crossterm::ErrorKind),
}

impl From<flutter_sys::Error> for Error {
    fn from(value: flutter_sys::Error) -> Self {
        Error::EngineError(value)
    }
}

impl From<crossterm::ErrorKind> for Error {
    fn from(value: crossterm::ErrorKind) -> Self {
        Error::TerminalError(value)
    }
}

fn draw_semantic_labels(
    terminal_window: &mut TerminalWindow,
    parent_merged_transform: FlutterTransformation,
    node: GraphNode,
) -> Result<(), crossterm::ErrorKind> {
    let current = node.current;

    let transform = current.transform.merge_with(&parent_merged_transform);

    if !current
        .flags
        .contains(&flutter_sys::FlutterSemanticsFlag::IsHidden)
        && !current.label.is_empty()
    {
        terminal_window.draw_text(
            (transform.transX * transform.scaleX).round() as usize,
            (transform.transY * transform.scaleY / 2.0).round() as usize,
            &current.label,
        )?;
    }

    for child in node.children {
        draw_semantic_labels(terminal_window, transform, child)?;
    }

    Ok(())
}
