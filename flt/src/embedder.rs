use crate::constants::{FPS, PIXEL_RATIO};
use crate::event::{EngineEvent, PlatformEvent};
use crate::semantics::FlutterSemanticsTree;
use crate::task_runner::TaskRunner;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{Callbacks, FlutterEngine};
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

impl TerminalEmbedder {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        simple_output: bool,
        debug_semantics: bool,
    ) -> Result<Self, Error> {
        let (main_sender, main_receiver) = channel();

        let terminal_window = TerminalWindow::new(simple_output, main_sender.clone());

        let callbacks = {
            let (sender_a, sender_b, sender_c, sender_d) = (
                main_sender.clone(),
                main_sender.clone(),
                main_sender.clone(),
                main_sender.clone(),
            );

            let platform_thread_id = thread::current().id();

            Callbacks {
                post_platform_task_callback: Some(Box::new(move |task| {
                    sender_a
                        .send(PlatformEvent::EngineEvent(EngineEvent::EngineTask(task)))
                        .unwrap();
                })),
                platform_task_runs_task_on_current_thread_callback: Some(Box::new(move || {
                    thread::current().id() == platform_thread_id
                })),
                log_message_callback: Some(Box::new(move |tag, message| {
                    sender_b
                        .send(PlatformEvent::EngineEvent(EngineEvent::LogMessage {
                            tag,
                            message,
                        }))
                        .unwrap();
                })),
                update_semantics_callback: Some(Box::new(move |updates| {
                    sender_c
                        .send(PlatformEvent::EngineEvent(EngineEvent::UpdateSemantics(
                            updates,
                        )))
                        .unwrap();
                })),
                draw_callback: Some(Box::new(move |pixel_grid| {
                    sender_d
                        .send(PlatformEvent::EngineEvent(EngineEvent::Draw(pixel_grid)))
                        .unwrap();
                })),
            }
        };

        let mut embedder = Self {
            engine: FlutterEngine::new(assets_dir, icu_data_path, callbacks)?,
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
            dimensions: (0, 0),
        };

        embedder.engine.notify_display_update(FPS as f64)?;
        embedder.reset_viewport()?;

        Ok(embedder)
    }

    pub(crate) fn reset_viewport(&mut self) -> Result<(), Error> {
        self.mouse_down_pos = (0, 0);
        self.prev_window_offset = (0, 0);
        self.window_offset = (0, 0);
        self.dimensions = self.terminal_window.size();

        self.engine
            .send_window_metrics_event(self.terminal_window.size(), PIXEL_RATIO)?;
        self.engine.schedule_frame()?;

        Ok(())
    }

    pub fn run_event_loop(&mut self) -> Result<(), Error> {
        let mut should_run = true;

        // TODO(jiahaog): Consider async Rust or Tokio instead.
        while should_run {
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

            // TODO(jiahaog): Doing it like this probably makes us only able to run expired
            // tasks when a platform event is received.
            self.platform_task_runner.run_expired_tasks(&self.engine)?;
        }

        Ok(())
    }
}
