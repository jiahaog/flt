use crate::sys;
pub enum KeyEventType {
    Up,
    Down,
    Repeat,
}

impl From<KeyEventType> for sys::FlutterKeyEventType {
    fn from(value: KeyEventType) -> Self {
        match value {
            KeyEventType::Up => sys::FlutterKeyEventType_kFlutterKeyEventTypeUp,
            KeyEventType::Down => sys::FlutterKeyEventType_kFlutterKeyEventTypeDown,
            KeyEventType::Repeat => sys::FlutterKeyEventType_kFlutterKeyEventTypeRepeat,
        }
    }
}
