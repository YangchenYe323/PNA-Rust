#![deny(missing_docs)]

//! This crate provides two Key-Value storage applications that implements thet `KvsEngine` trait,
//! one is `KvStore`, which uses log-structured file under the hood, and the other is `SledKvsEngine`,
//! a wrapper around `sled::Db` structure.
//!
//! Besides, the crates also provides a Server/Client utility built on top of `KvsEngine` that let user set
//! up a network service for their Key-Value storage application

mod error;
mod network;
mod storage;

#[macro_use]
extern crate failure;
pub use error::KVError;
pub use error::KVErrorKind;
pub use network::{Command, KvClient, KvServer, Response};
pub use storage::{KvStore, KvsEngine, SledKvsEngine};

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;
