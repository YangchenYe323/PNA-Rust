use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::io;

/// Error Type for the KV Project
#[derive(Debug)]
pub struct KVError {
    inner: Context<KVErrorKind>,
}

/// Kinds of possible Errors in KV Project
#[derive(Debug, Fail)]
pub enum KVErrorKind {
    /// Try to remove a non-existent key
    #[fail(display = "Key not found")]
    KeyNotFound(String),

    /// IoError triggered by file I/Os
    #[fail(display = "Io Error: {}", _0)]
    IoError(#[cause] io::Error),

    /// Errors when the data associated with a key is not a Set Command
    #[fail(display = "Unexpected Command Type for key {}", _0)]
    UnexpectedCommandType(String),

    /// Serialization/Deserialization Error triggered by serde
    #[fail(display = "Json parsing error: {}", _0)]
    JsonError(#[cause] serde_json::Error),

    /// Error triggered by sled engine
    // todo: try to find a better way to convert sled::Error to KVError
    #[fail(display = "Sled Error: {}", _0)]
    SledError(#[cause] sled::Error),

    /// ThreadPool Panic Error
    #[fail(display = "ThreadPool thread Panicked")]
    ThreadPanic,

    /// Rayon related error
    #[fail(display = "Rayon ThreadPool Error")]
    RayonError,

    /// String Error
    #[fail(display = "{}", _0)]
    StringError(String),
}

impl Fail for KVError {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for KVError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.inner, f)
    }
}

impl From<KVErrorKind> for KVError {
    fn from(kind: KVErrorKind) -> KVError {
        KVError {
            inner: Context::new(kind),
        }
    }
}

impl From<Context<KVErrorKind>> for KVError {
    fn from(context: Context<KVErrorKind>) -> KVError {
        KVError { inner: context }
    }
}

impl From<io::Error> for KVError {
    fn from(error: io::Error) -> KVError {
        KVErrorKind::IoError(error).into()
    }
}

impl From<serde_json::Error> for KVError {
    fn from(error: serde_json::Error) -> KVError {
        KVErrorKind::JsonError(error).into()
    }
}

impl From<sled::Error> for KVError {
    fn from(error: sled::Error) -> KVError {
        KVErrorKind::SledError(error).into()
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for KVError {
    fn from(_error: tokio::sync::oneshot::error::RecvError) -> KVError {
        KVErrorKind::StringError("Sync Error".to_string()).into()
    }
}
