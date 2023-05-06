use std::fmt::Display;

#[derive(Debug)]
pub enum Error {
    EngineError(flutter_sys::Error),
    TerminalError(crossterm::ErrorKind),
}

impl From<flutter_sys::Error> for Error {
    fn from(value: flutter_sys::Error) -> Self {
        Error::EngineError(value)
    }
}

impl From<crossterm::ErrorKind> for Error {
    fn from(value: crossterm::ErrorKind) -> Self {
        Error::TerminalError(value)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(match self {
            Error::EngineError(e) => e,
            Error::TerminalError(e) => e,
        })
    }
}
