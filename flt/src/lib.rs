use crossterm::event::{read, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use flutter_sys::{
    EmbedderCallbacks, Pixel, SafeEngine, SafeMouseButton, SafePointerPhase, SafeSignalKind,
};
use terminal_window::TerminalWindow;

mod terminal_window;

const FPS: usize = 60;
const PIXEL_RATIO: f64 = 0.7;
// Number of pixel for each scroll event as the terminal doesn't tell us how
// many lines the mouse has scrolled by.
const SCROLL_DELTA: f64 = 10.0;

pub struct TerminalEmbedder {
    engine: SafeEngine<TerminalEmbedderCallbacks>,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str) -> Self {
        let callbacks = TerminalEmbedderCallbacks {
            terminal_window: TerminalWindow::new(),
        };

        let (width, height) = callbacks.terminal_window.size();

        let embedder = Self {
            engine: SafeEngine::new(assets_dir, icu_data_path, callbacks),
        };

        embedder.engine.notify_display_update(FPS as f64);

        embedder
            .engine
            .send_window_metrics_event(width, height, PIXEL_RATIO);

        embedder
    }

    pub fn wait_for_input(&self) {
        loop {
            match read().unwrap() {
                crossterm::event::Event::FocusGained => todo!(),
                crossterm::event::Event::FocusLost => todo!(),
                crossterm::event::Event::Key(KeyEvent {
                    code, modifiers, ..
                }) => {
                    if code == KeyCode::Char('c') && modifiers == KeyModifiers::CONTROL {
                        break;
                    }
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
                                SafePointerPhase::Down,
                                column as f64,
                                row as f64,
                                SafeSignalKind::None,
                                0.0,
                                vec![to_mouse_button(mouse_button)],
                            );
                        }
                        crossterm::event::MouseEventKind::Up(mouse_button) => {
                            self.engine.send_pointer_event(
                                SafePointerPhase::Up,
                                column as f64,
                                row as f64,
                                SafeSignalKind::None,
                                0.0,
                                vec![to_mouse_button(mouse_button)],
                            );
                        }
                        // Just continue as it's too annoying to log these common events.
                        crossterm::event::MouseEventKind::Drag(_) => continue,
                        crossterm::event::MouseEventKind::Moved => {
                            self.engine.send_pointer_event(
                                SafePointerPhase::Hover,
                                column as f64,
                                row as f64,
                                SafeSignalKind::None,
                                0.0,
                                vec![],
                            );
                        }
                        crossterm::event::MouseEventKind::ScrollUp => {
                            self.engine.send_pointer_event(
                                SafePointerPhase::Up,
                                column as f64,
                                row as f64,
                                SafeSignalKind::Scroll,
                                -SCROLL_DELTA,
                                vec![],
                            );
                        }
                        crossterm::event::MouseEventKind::ScrollDown => {
                            self.engine.send_pointer_event(
                                SafePointerPhase::Down,
                                column as f64,
                                row as f64,
                                SafeSignalKind::Scroll,
                                SCROLL_DELTA,
                                vec![],
                            );
                        }
                    };
                }
                crossterm::event::Event::Paste(_) => todo!(),
                crossterm::event::Event::Resize(columns, rows) => {
                    self.engine.send_window_metrics_event(
                        columns as usize,
                        // The terminal renderer merges two pixels (top and bottom) into one.
                        (rows * 2) as usize,
                        PIXEL_RATIO,
                    );
                }
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

fn to_mouse_button(value: crossterm::event::MouseButton) -> SafeMouseButton {
    match value {
        crossterm::event::MouseButton::Left => SafeMouseButton::Left,
        crossterm::event::MouseButton::Right => SafeMouseButton::Right,
        crossterm::event::MouseButton::Middle => SafeMouseButton::Middle,
    }
}
