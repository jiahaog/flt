use crossterm::event::{read, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use flutter_sys::{
    EmbedderCallbacks, FlutterEngine, FlutterPointerMouseButton, FlutterPointerPhase,
    FlutterPointerSignalKind, KeyEventType, Pixel,
};
use terminal_window::TerminalWindow;

mod terminal_window;

const FPS: usize = 60;
const PIXEL_RATIO: f64 = 0.7;
// Number of pixel for each scroll event as the terminal doesn't tell us how
// many lines the mouse has scrolled by.
const SCROLL_DELTA: f64 = 10.0;

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

    pub fn wait_for_input(&mut self) -> Result<(), Error> {
        self.engine.run(|| {
            // self.read_input()?;
            Ok(())
        })?;

        Ok(())
    }

    fn read_input(&self) -> Result<(), flutter_sys::Error> {
        // TODO poll
        match read().unwrap() {
            crossterm::event::Event::FocusGained => todo!(),
            crossterm::event::Event::FocusLost => todo!(),
            crossterm::event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
                    return Ok(());
                }
                if let KeyCode::Char(c) = code {
                    self.engine.send_key_event(KeyEventType::Down, c)?;
                    self.engine.send_key_event(KeyEventType::Up, c)?;
                }
                Ok(())
            }
            crossterm::event::Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers: _,
            }) => {
                // The terminal renderer merges two pixels (top and bottom) into one.
                let row = row * 2;

                match kind {
                    crossterm::event::MouseEventKind::Down(mouse_button) => {
                        // (SafePointerPhase::Down, to_mouse_button(mouse_button))
                        self.engine.send_pointer_event(
                            FlutterPointerPhase::Down,
                            column as f64,
                            row as f64,
                            FlutterPointerSignalKind::None,
                            0.0,
                            vec![to_mouse_button(mouse_button)],
                        )?;
                        Ok(())
                    }
                    crossterm::event::MouseEventKind::Up(mouse_button) => {
                        self.engine.send_pointer_event(
                            FlutterPointerPhase::Up,
                            column as f64,
                            row as f64,
                            FlutterPointerSignalKind::None,
                            0.0,
                            vec![to_mouse_button(mouse_button)],
                        )?;
                        Ok(())
                    }
                    // Just continue as it's too annoying to log these common events.
                    crossterm::event::MouseEventKind::Drag(_) => Ok(()),
                    crossterm::event::MouseEventKind::Moved => {
                        self.engine.send_pointer_event(
                            FlutterPointerPhase::Hover,
                            column as f64,
                            row as f64,
                            FlutterPointerSignalKind::None,
                            0.0,
                            vec![],
                        )?;
                        Ok(())
                    }
                    crossterm::event::MouseEventKind::ScrollUp => {
                        self.engine.send_pointer_event(
                            FlutterPointerPhase::Up,
                            column as f64,
                            row as f64,
                            FlutterPointerSignalKind::Scroll,
                            -SCROLL_DELTA,
                            vec![],
                        )?;
                        Ok(())
                    }
                    crossterm::event::MouseEventKind::ScrollDown => {
                        self.engine.send_pointer_event(
                            FlutterPointerPhase::Down,
                            column as f64,
                            row as f64,
                            FlutterPointerSignalKind::Scroll,
                            SCROLL_DELTA,
                            vec![],
                        )?;
                        Ok(())
                    }
                }
            }
            crossterm::event::Event::Paste(_) => todo!(),
            crossterm::event::Event::Resize(columns, rows) => {
                self.engine.send_window_metrics_event(
                    columns as usize,
                    // The terminal renderer merges two pixels (top and bottom) into one.
                    (rows * 2) as usize,
                    // TODO(jiahaog): Choose a pixel ratio based on the size so everything is not so compressed?
                    PIXEL_RATIO,
                )?;
                Ok(())
            }
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
}

fn to_mouse_button(value: crossterm::event::MouseButton) -> FlutterPointerMouseButton {
    match value {
        crossterm::event::MouseButton::Left => FlutterPointerMouseButton::Left,
        crossterm::event::MouseButton::Right => FlutterPointerMouseButton::Right,
        crossterm::event::MouseButton::Middle => FlutterPointerMouseButton::Middle,
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
