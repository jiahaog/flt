use crossterm::event::read;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use flutter_sys::EmbedderCallbacks;
use flutter_sys::Pixel;
use flutter_sys::SafeEngine;
use flutter_sys::SafeMouseButton;
use flutter_sys::SafePointerPhase;
use std::cell::RefCell;
use std::rc::Rc;
use terminal_window::TerminalWindow;

mod terminal_window;

const FPS: usize = 60;

pub struct TerminalEmbedder {
    engine: SafeEngine<TerminalEmbedderCallbacks>,
    terminal_window: Rc<RefCell<TerminalWindow>>,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str) -> Self {
        let terminal_window = Rc::new(RefCell::new(TerminalWindow::new()));

        let callbacks = TerminalEmbedderCallbacks {
            embedder: terminal_window.clone(),
        };

        let embedder = Self {
            terminal_window: terminal_window.clone(),
            engine: SafeEngine::new(assets_dir, icu_data_path, callbacks),
        };

        embedder.engine.notify_display_update(FPS as f64);

        let (width, height) = terminal_window.borrow().size();
        embedder.engine.send_window_metrics_event(width, height);

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

                    let (phase, button) = match kind {
                        crossterm::event::MouseEventKind::Down(mouse_button) => {
                            (SafePointerPhase::Down, to_mouse_button(mouse_button))
                        }
                        crossterm::event::MouseEventKind::Up(mouse_button) => {
                            (SafePointerPhase::Up, to_mouse_button(mouse_button))
                        }
                        // Just continue as it's too annoying to log these common events.
                        crossterm::event::MouseEventKind::Drag(_) => continue,
                        crossterm::event::MouseEventKind::Moved => continue,
                        kind => {
                            println!("ignoring event {kind:?}");
                            continue;
                        }
                    };

                    self.engine
                        .send_pointer_event(phase, column as f64, row as f64, vec![button]);
                }
                crossterm::event::Event::Paste(_) => todo!(),
                crossterm::event::Event::Resize(columns, rows) => {
                    self.engine.send_window_metrics_event(
                        columns as usize,
                        // The terminal renderer merges two pixels (top and bottom) into one.
                        (rows * 2) as usize,
                    );
                }
            }
        }
    }
}

struct TerminalEmbedderCallbacks {
    embedder: Rc<RefCell<TerminalWindow>>,
}

impl EmbedderCallbacks for TerminalEmbedderCallbacks {
    fn log(&self, tag: String, message: String) {
        // TODO: Print to the main terminal.
        println!("{tag}: {message}");
    }

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) {
        self.embedder
            .borrow_mut()
            .draw(width, height, buffer)
            .unwrap()
    }

    fn size(&self) -> (usize, usize) {
        self.embedder.borrow().size()
    }
}

fn to_mouse_button(value: crossterm::event::MouseButton) -> SafeMouseButton {
    match value {
        crossterm::event::MouseButton::Left => SafeMouseButton::Left,
        crossterm::event::MouseButton::Right => SafeMouseButton::Right,
        crossterm::event::MouseButton::Middle => SafeMouseButton::Middle,
    }
}
