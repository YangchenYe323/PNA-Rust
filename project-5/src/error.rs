use failure::{Backtrace, Context, Fail};
use std::fmt;
use std::io;

/// Error Type for the KV Project
#[derive(Debug)]
pub struct KVError {
    inner: Context<KVErrorKind>,
}

/// Kinds of possible Errors in KV Project
#[derive(Debug, Clone, Copy, PartialEq, Eq, Fail)]
pub enum KVErrorKind {
    /// Try to remove a non-existent key
    #[fail(display = "Key not found")]
    KeyNotFound,
    /// IoError triggered by file I/Os
    #[fail(display = "Io Error")]
    IoError,
    /// Errors when the data associated with a key is not a Set Command
    #[fail(display = "Unexpected Command Type for key")]
    UnexpectedCommandType,
    /// Serialization/Deserialization Error triggered by serde
    #[fail(display = "Json parsing error")]
    JsonError,
    /// Error triggered by sled engine
    #[fail(display = "Sled Error")]
    SledError,
    /// ThreadPool Panic Error
    #[fail(display = "ThreadPool thread Panicked")]
    ThreadPanic,
    /// Rayon related error
    #[fail(display = "Rayon ThreadPool Error")]
    RayonError,
    /// Tokio Channel Sync Error
    #[fail(display = "Tokio Channel Syn Error")]
    TokioSyncError,
    /// PlaceHolder for Unknown Error
    #[fail(display = "Unknwon Error")]
    UnknownError,
}

impl KVError {
    /// get the kind of the error
    pub fn kind(&self) -> KVErrorKind {
        *self.inner.get_context()
    }
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
        error.context(KVErrorKind::IoError).into()
    }
}

impl From<serde_json::Error> for KVError {
    fn from(error: serde_json::Error) -> KVError {
        error.context(KVErrorKind::JsonError).into()
    }
}

impl From<sled::Error> for KVError {
    fn from(error: sled::Error) -> KVError {
        error.context(KVErrorKind::SledError).into()
    }
}

impl From<tokio::sync::oneshot::error::RecvError> for KVError {
    fn from(error: tokio::sync::oneshot::error::RecvError) -> KVError {
        error.context(KVErrorKind::TokioSyncError).into()
    }
}
