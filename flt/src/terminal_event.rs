use crate::constants::{PIXEL_RATIO, SCROLL_DELTA};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use flutter_sys::{
    FlutterEngine, FlutterPointerMouseButton, FlutterPointerPhase, FlutterPointerSignalKind,
    KeyEventType,
};

/// Returns whether the process should terminate.
pub(crate) fn handle_terminal_event(
    engine: &FlutterEngine,
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
            if let KeyCode::Char(c) = code {
                engine.send_key_event(KeyEventType::Down, c)?;
                engine.send_key_event(KeyEventType::Up, c)?;
            }
            Ok(true)
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
                    engine.send_pointer_event(
                        FlutterPointerPhase::Down,
                        column as f64,
                        row as f64,
                        FlutterPointerSignalKind::None,
                        0.0,
                        vec![to_mouse_button(mouse_button)],
                    )?;
                    Ok(true)
                }
                crossterm::event::MouseEventKind::Up(mouse_button) => {
                    engine.send_pointer_event(
                        FlutterPointerPhase::Up,
                        column as f64,
                        row as f64,
                        FlutterPointerSignalKind::None,
                        0.0,
                        vec![to_mouse_button(mouse_button)],
                    )?;
                    Ok(true)
                }
                // Just continue as it's too annoying to log these common events.
                crossterm::event::MouseEventKind::Drag(_) => Ok(true),
                crossterm::event::MouseEventKind::Moved => {
                    engine.send_pointer_event(
                        FlutterPointerPhase::Hover,
                        column as f64,
                        row as f64,
                        FlutterPointerSignalKind::None,
                        0.0,
                        vec![],
                    )?;
                    Ok(true)
                }
                crossterm::event::MouseEventKind::ScrollUp => {
                    engine.send_pointer_event(
                        FlutterPointerPhase::Up,
                        column as f64,
                        row as f64,
                        FlutterPointerSignalKind::Scroll,
                        -SCROLL_DELTA,
                        vec![],
                    )?;
                    Ok(true)
                }
                crossterm::event::MouseEventKind::ScrollDown => {
                    engine.send_pointer_event(
                        FlutterPointerPhase::Down,
                        column as f64,
                        row as f64,
                        FlutterPointerSignalKind::Scroll,
                        SCROLL_DELTA,
                        vec![],
                    )?;
                    Ok(true)
                }
            }
        }
        crossterm::event::Event::Paste(_) => todo!(),
        crossterm::event::Event::Resize(columns, rows) => {
            engine.send_window_metrics_event(
                columns as usize,
                // The terminal renderer merges two pixels (top and bottom) into one.
                (rows * 2) as usize,
                // TODO(jiahaog): Choose a pixel ratio based on the size so everything is not so compressed?
                PIXEL_RATIO,
            )?;
            Ok(true)
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
