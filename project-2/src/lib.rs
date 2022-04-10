#![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs in memory

mod error;
mod kvstore;
#[macro_use]
extern crate failure;
pub use error::KVError;
pub use error::KVErrorKind;
pub use kvstore::KvStore;

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;
