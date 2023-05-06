use std::fmt::Display;

use crate::sys;

#[derive(Debug)]
pub enum Error {
    InvalidLibraryVersion,
    InvalidArguments,
    InternalConsistency,
}

impl From<sys::FlutterEngineResult> for Error {
    fn from(value: sys::FlutterEngineResult) -> Self {
        match value {
            sys::FlutterEngineResult_kInvalidLibraryVersion => Error::InvalidLibraryVersion,
            sys::FlutterEngineResult_kInvalidArguments => Error::InvalidArguments,
            sys::FlutterEngineResult_kInternalInconsistency => Error::InternalConsistency,
            value => panic!("Unexpected value for FlutterEngineResult: {} ", value),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}
