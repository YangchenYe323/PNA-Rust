#![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs in memory

mod kvstore;
mod err;
#[macro_use] extern crate failure;
pub use kvstore::KvStore;
pub use err::KVError;
pub use err::KVErrorKind;

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;