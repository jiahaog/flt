use crossterm::event::read;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use engine::Embedder;
use engine::Pixel;
use engine::SafeEngine;
use engine::SafePointerPhase;
use std::cell::RefCell;
use std::rc::Rc;
use terminal_window::TerminalWindow;

mod engine;
mod terminal_window;

const FPS: usize = 60;

struct TerminalEmbedderImpl {
    outside: Rc<RefCell<TerminalWindow>>,
}

pub struct TerminalEmbedder {
    corruption_token: String,
    engine: SafeEngine<TerminalEmbedderImpl>,
    terminal_window: Rc<RefCell<TerminalWindow>>,
}

impl TerminalEmbedder {
    pub fn new(assets_dir: &str, icu_data_path: &str) -> Self {
        let terminal_window = Rc::new(RefCell::new(TerminalWindow::new("terminal".to_string())));

        let embedder = TerminalEmbedderImpl {
            outside: terminal_window.clone(),
        };

        let terminal_embedder = Self {
            corruption_token: "user_data".to_string(),
            terminal_window: terminal_window.clone(),
            engine: SafeEngine::new(assets_dir, icu_data_path, embedder),
        };

        terminal_embedder.engine.notify_display_update(FPS as f64);

        let (width, height) = terminal_window.borrow().size();
        terminal_embedder
            .engine
            .send_window_metrics_event(width, height);

        terminal_embedder
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
                            (SafePointerPhase::Down, mouse_button.into())
                        }
                        crossterm::event::MouseEventKind::Up(mouse_button) => {
                            (SafePointerPhase::Up, mouse_button.into())
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

impl Embedder for TerminalEmbedderImpl {
    fn log(&self, tag: String, message: String) {
        // TODO: Print to the main terminal.
        println!("{tag}: {message}");
    }

    fn draw(&mut self, width: usize, height: usize, buffer: Vec<Pixel>) {
        self.outside
            .borrow_mut()
            .draw(width, height, buffer)
            .unwrap()
    }

    fn size(&self) -> (usize, usize) {
        self.outside.borrow().size()
    }
}
