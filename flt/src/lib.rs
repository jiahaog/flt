use constants::{FPS, PIXEL_RATIO};

use flutter_sys::{task_runner::Task, EmbedderCallbacks, FlutterEngine, Pixel};
use terminal_event_task::TerminalEventTask;
use terminal_window::TerminalWindow;

mod constants;
mod terminal_event_task;
mod terminal_window;

pub struct TerminalEmbedder {
    engine: FlutterEngine<TerminalEmbedderCallbacks>,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str, simple_output: bool) -> Result<Self, Error> {
        let callbacks = TerminalEmbedderCallbacks {
            terminal_window: TerminalWindow::new(simple_output),
        };

        let (width, height) = callbacks.terminal_window.size();

        let embedder = Self {
            engine: FlutterEngine::new(assets_dir, icu_data_path, callbacks)?,
        };
        embedder.engine.notify_display_update(FPS as f64)?;
        embedder.engine.update_semantics(true)?;

        embedder
            .engine
            .send_window_metrics_event(width, height, PIXEL_RATIO)?;

        Ok(embedder)
    }

    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        let task_runner = self.engine.get_task_runner();

        loop {
            // TODO(jiahaog): Make this work instead of the following line.
            // task_runner.post_task(TerminalEventTask {});
            TerminalEventTask {}.run(&self.engine)?;

            task_runner.run_expired_tasks(&self.engine)?;
        }
    }
}

struct TerminalEmbedderCallbacks {
    terminal_window: TerminalWindow,
}

impl EmbedderCallbacks for TerminalEmbedderCallbacks {
    fn log(&self, tag: String, message: String) {
        // TODO: Print to the main terminal.
        println!("{tag}: {message}");
    }

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) {
        self.terminal_window.draw(width, height, buffer).unwrap()
    }

    fn draw_text(&mut self, x: usize, y: usize, text: &str) {
        self.terminal_window.draw_text(x, y, text).unwrap()
    }
}

#[derive(Debug)]
pub enum Error {
    EngineError(flutter_sys::Error),
}

impl From<flutter_sys::Error> for Error {
    fn from(value: flutter_sys::Error) -> Self {
        Error::EngineError(value)
    }
}
