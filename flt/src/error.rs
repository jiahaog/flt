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
