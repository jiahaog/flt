use crate::constants::FPS;
use crate::event::{EngineEvent, PlatformEvent};
use crate::semantics::FlutterSemanticsTree;
use crate::task_runner::TaskRunner;
use crate::terminal_window::TerminalWindow;
use crate::Error;
use flutter_sys::{Callbacks, FlutterEngine};
use std::sync::mpsc::{channel, Receiver};
use std::thread;

pub struct TerminalEmbedder {
    pub(crate) engine: FlutterEngine,
    pub(crate) semantics_tree: FlutterSemanticsTree,
    pub(crate) terminal_window: TerminalWindow,

    // Switches provided at startup.
    // TODO(jiahaog): This should be a path instead.
    pub(crate) debug_semantics: bool,
    pub(crate) show_semantics: bool,

    // Event related.
    pub(crate) should_run: bool,
    pub(crate) platform_events: Receiver<PlatformEvent>,
    pub(crate) platform_task_runner: TaskRunner,

    // Window related.
    pub(crate) dimensions: (usize, usize),
    pub(crate) zoom: f64,
    pub(crate) scale: f64,
    pub(crate) window_offset: (isize, isize),
    pub(crate) prev_window_offset: (isize, isize),
    pub(crate) mouse_down_pos: (isize, isize),
}

impl TerminalEmbedder {
    pub fn new(
        assets_dir: &str,
        icu_data_path: &str,
        simple_output: bool,
        alternate_screen: bool,
        log_events: bool,
        debug_semantics: bool,
    ) -> Result<Self, Error> {
        let (main_sender, main_receiver) = channel();

        let terminal_window = TerminalWindow::new(
            simple_output,
            alternate_screen,
            log_events,
            main_sender.clone(),
        );

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
            terminal_window,
            semantics_tree: FlutterSemanticsTree::new(),
            debug_semantics,
            show_semantics: false,
            should_run: true,
            platform_events: main_receiver,
            platform_task_runner: TaskRunner::new(),
            dimensions: (0, 0),
            zoom: 1.0,
            scale: 1.0,
            window_offset: (0, 0),
            prev_window_offset: (0, 0),
            mouse_down_pos: (0, 0),
        };

        embedder.engine.notify_display_update(FPS as f64)?;
        embedder.reset_viewport()?;

        // This event sets the engine window dimensions which will kickstart rendering.
        main_sender
            .send(PlatformEvent::EngineEvent(EngineEvent::Draw(vec![])))
            .unwrap();

        Ok(embedder)
    }

    pub(crate) fn reset_viewport(&mut self) -> Result<(), Error> {
        self.dimensions = self.terminal_window.size();
        self.zoom = 1.0;
        self.scale = 1.0;
        self.window_offset = (0, 0);
        self.prev_window_offset = (0, 0);
        self.mouse_down_pos = (0, 0);

        self.engine.schedule_frame()?;
        Ok(())
    }
}
