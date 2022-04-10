#![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs in memory

mod kvstore;
#[macro_use]
extern crate failure;
use failure::Error;
pub use kvstore::KvStore;

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, Error>;
