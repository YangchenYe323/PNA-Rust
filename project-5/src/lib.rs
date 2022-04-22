#![deny(missing_docs)]
#![warn(rust_2018_idioms)]


//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs

mod error;
// mod network;
mod storage;
pub mod thread_pool;

#[macro_use]
extern crate failure;
pub use error::KVError;
pub use error::KVErrorKind;
// pub use network::{Command, KvClient, KvServer, Response};
pub use storage::{KvStore, KvsEngine, SledKvsEngine};

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;
