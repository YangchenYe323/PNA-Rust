#![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs

mod error;
mod storage;
mod network;
pub mod thread_pool;

#[macro_use]
extern crate failure;
pub use error::KVError;
pub use error::KVErrorKind;
pub use storage::{ KvsEngine, KvStore, SledKvsEngine };
pub use network::{ KvClient, KvServer, Response, Command };

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;
