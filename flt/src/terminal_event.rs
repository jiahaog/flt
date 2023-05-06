use crate::{
    constants::{PIXEL_RATIO, SCROLL_DELTA, ZOOM_FACTOR},
    TerminalEmbedder,
};
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use flutter_sys::{FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind};

impl TerminalEmbedder {
    /// Returns whether the process should terminate.
    pub(crate) fn handle_terminal_event(
        &mut self,
        event: Event,
    ) -> Result<bool, flutter_sys::Error> {
        match event {
            crossterm::event::Event::FocusGained => todo!(),
            crossterm::event::Event::FocusLost => todo!(),
            crossterm::event::Event::Key(KeyEvent {
                code, modifiers, ..
            }) => {
                if modifiers == KeyModifiers::CONTROL && code == KeyCode::Char('c') {
                    return Ok(false);
                }
                if modifiers == KeyModifiers::ALT && code == KeyCode::Char('s') {
                    self.show_semantics = !self.show_semantics;
                    // Flutter does not update the semantics callback when they are disabled.
                    if !self.show_semantics {
                        self.terminal_window.update_semantics(vec![]);
                    }
                    self.engine.update_semantics(self.show_semantics)?;
                    return Ok(true);
                }
                // TODO(jiahaog): Implement keyboard support.
                Ok(true)
            }
            crossterm::event::Event::Mouse(MouseEvent {
                kind,
                column,
                row,
                modifiers,
            }) => {
                if modifiers.contains(KeyModifiers::ALT) {
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
                        }
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                            self.zoom = if kind == MouseEventKind::ScrollUp {
                                self.zoom * ZOOM_FACTOR
                            } else {
                                self.zoom / ZOOM_FACTOR
                            };

                            // TODO(jiahaog): Zoom towards the cursor instead of the top left.
                            self.engine.send_window_metrics_event(
                                (
                                    (self.dimensions.0 as f64 * self.zoom).round() as usize,
                                    (self.dimensions.1 as f64 * self.zoom).round() as usize,
                                ),
                                PIXEL_RATIO * self.zoom,
                            )?;
                        }
                        _ => (),
                    }
                } else {
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
                Ok(true)
            }
            crossterm::event::Event::Paste(_) => todo!(),
            crossterm::event::Event::Resize(columns, rows) => {
                self.engine
                    .send_window_metrics_event((columns as usize, rows as usize), PIXEL_RATIO)?;
                Ok(true)
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
