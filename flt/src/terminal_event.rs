use crate::{
    constants::{SCROLL_DELTA, ZOOM_FACTOR},
    Error, TerminalEmbedder,
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use flutter_sys::{FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind};

/// Modifier to intercept events which will not be forwarded to Flutter.
const INTERCEPT_MODIFIER: KeyModifiers = KeyModifiers::CONTROL;

impl TerminalEmbedder {
    pub(crate) fn handle_terminal_event(&mut self, event: Event) -> Result<(), Error> {
        match event {
            crossterm::event::Event::FocusGained => todo!(),
            crossterm::event::Event::FocusLost => todo!(),
            crossterm::event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                if modifiers == INTERCEPT_MODIFIER && code == KeyCode::Char('c') {
                    self.should_run = false;
                    return Ok(());
                }
                if modifiers == INTERCEPT_MODIFIER && code == KeyCode::Char('z') {
                    self.show_semantics = !self.show_semantics;
                    // Flutter does not update the semantics callback when they are disabled.
                    if !self.show_semantics {
                        self.terminal_window.update_semantics(vec![]);
                    }
                    self.engine.update_semantics(self.show_semantics)?;
                    return Ok(());
                }
                if modifiers == INTERCEPT_MODIFIER && code == KeyCode::Char('r') {
                    self.reset_viewport()?;
                    return Ok(());
                }
                if modifiers == INTERCEPT_MODIFIER
                    && (code == KeyCode::Char('w') || code == KeyCode::Char('s'))
                {
                    self.scale = if code == KeyCode::Char('w') {
                        self.scale * ZOOM_FACTOR
                    } else {
                        self.scale / ZOOM_FACTOR
                    };

                    return Ok(());
                }

                // TODO(jiahaog): Implement keyboard support.
                Ok(())
            }
            crossterm::event::Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
            }) => {
                if modifiers == INTERCEPT_MODIFIER {
                    match kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            self.mouse_down_pos = (column as isize, row as isize);
                            self.prev_window_offset = self.window_offset;
                        }
                        MouseEventKind::Drag(MouseButton::Left) => {
                            let delta = (
                                column as isize - self.mouse_down_pos.0,
                                row as isize - self.mouse_down_pos.1,
                            );
                            self.window_offset = (
                                // Negate delta because when mouse is moved to the right
                                // (positive delta), the terminal needs to be offset to the
                                // left (negative offset) to create the illusion of panning the
                                // window it.
                                self.prev_window_offset.0 - delta.0,
                                self.prev_window_offset.1 - delta.1,
                            );

                            self.engine.schedule_frame()?;
                        }
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                            self.zoom = if kind == MouseEventKind::ScrollUp {
                                self.zoom * ZOOM_FACTOR
                            } else {
                                self.zoom / ZOOM_FACTOR
                            };

                            self.engine.schedule_frame()?;

                            // TODO(jiahaog): Zoom towards the cursor instead of
                            // the top left.
                        }
                        _ => (),
                    }
                } else {
                    let (column, row) = (
                        column as f64 + self.window_offset.0 as f64,
                        row as f64 + self.window_offset.1 as f64,
                    );
                    match kind {
                        crossterm::event::MouseEventKind::Down(mouse_button) => {
                            self.engine.send_pointer_event(
                                FlutterPointerPhase::Down,
                                (column as f64, row as f64),
                                FlutterPointerSignalKind::None,
                                0.0,
                                vec![to_mouse_button(mouse_button)],
                            )?;
                        }
                        crossterm::event::MouseEventKind::Up(mouse_button) => {
                            self.engine.send_pointer_event(
                                FlutterPointerPhase::Up,
                                (column as f64, row as f64),
                                FlutterPointerSignalKind::None,
                                0.0,
                                vec![to_mouse_button(mouse_button)],
                            )?;
                        }
                        crossterm::event::MouseEventKind::Drag(_) => (),
                        crossterm::event::MouseEventKind::Moved => {
                            self.engine.send_pointer_event(
                                FlutterPointerPhase::Hover,
                                (column as f64, row as f64),
                                FlutterPointerSignalKind::None,
                                0.0,
                                vec![],
                            )?;
                        }
                        crossterm::event::MouseEventKind::ScrollUp => {
                            self.engine.send_pointer_event(
                                FlutterPointerPhase::Up,
                                (column as f64, row as f64),
                                FlutterPointerSignalKind::Scroll,
                                -SCROLL_DELTA,
                                vec![],
                            )?;
                        }
                        crossterm::event::MouseEventKind::ScrollDown => {
                            self.engine.send_pointer_event(
                                FlutterPointerPhase::Down,
                                (column as f64, row as f64),
                                FlutterPointerSignalKind::Scroll,
                                SCROLL_DELTA,
                                vec![],
                            )?;
                        }
                    }
                }
                Ok(())
            }
            crossterm::event::Event::Paste(_) => todo!(),
            crossterm::event::Event::Resize(columns, rows) => {
                self.dimensions = (columns as usize, rows as usize);
                self.engine.schedule_frame()?;
                Ok(())
            }
        }
    }
}

fn to_mouse_button(value: crossterm::event::MouseButton) -> FlutterPointerMouseButton {
    match value {
        crossterm::event::MouseButton::Left => FlutterPointerMouseButton::Left,
        crossterm::event::MouseButton::Right => FlutterPointerMouseButton::Right,
        crossterm::event::MouseButton::Middle => FlutterPointerMouseButton::Middle,
    }
}
