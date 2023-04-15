use crate::sys;

pub enum FlutterPointerPhase {
    Up,
    Down,
    Hover,
}

impl From<FlutterPointerPhase> for sys::FlutterPointerPhase {
    fn from(value: FlutterPointerPhase) -> Self {
        match value {
            FlutterPointerPhase::Up => sys::FlutterPointerPhase_kUp,
            FlutterPointerPhase::Down => sys::FlutterPointerPhase_kDown,
            FlutterPointerPhase::Hover => sys::FlutterPointerPhase_kHover,
        }
    }
}

pub enum FlutterPointerSignalKind {
    None,
    Scroll,
}

impl From<FlutterPointerSignalKind> for sys::FlutterPointerSignalKind {
    fn from(value: FlutterPointerSignalKind) -> Self {
        match value {
            FlutterPointerSignalKind::None => {
                sys::FlutterPointerSignalKind_kFlutterPointerSignalKindNone
            }
            FlutterPointerSignalKind::Scroll => {
                sys::FlutterPointerSignalKind_kFlutterPointerSignalKindScroll
            }
        }
    }
}

pub enum FlutterPointerMouseButton {
    Left,
    Right,
    Middle,
}

impl From<FlutterPointerMouseButton> for sys::FlutterPointerMouseButtons {
    fn from(value: FlutterPointerMouseButton) -> Self {
        match value {
            FlutterPointerMouseButton::Left => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMousePrimary
            }
            FlutterPointerMouseButton::Right => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMouseSecondary
            }
            FlutterPointerMouseButton::Middle => {
                sys::FlutterPointerMouseButtons_kFlutterPointerButtonMouseMiddle
            }
        }
    }
}
