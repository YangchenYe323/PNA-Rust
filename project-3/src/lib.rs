#![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs in memory

mod error;
mod kvstore;
mod server;
mod client;
mod engine;
pub(crate) mod protocol;

#[macro_use]
extern crate failure;
pub use error::KVError;
pub use error::KVErrorKind;
pub use kvstore::KvStore;
pub use server::KvServer;
pub use server::Command;
pub use server::Response;
pub use client::KvClient;
pub use engine::KvsEngine;

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;
