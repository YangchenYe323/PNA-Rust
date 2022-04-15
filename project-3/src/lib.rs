// #![deny(missing_docs)]

//! This crate provides a KvStore structure
//! that is capable of storing key-value pairs in memory

mod client;
mod engine;
mod error;
mod kvsled;
mod kvstore;
pub(crate) mod protocol;
mod server;

#[macro_use]
extern crate failure;
pub use client::KvClient;
pub use engine::KvsEngine;
pub use error::KVError;
pub use error::KVErrorKind;
pub use kvsled::SledKvsEngine;
pub use kvstore::KvStore;
pub use server::Command;
pub use server::KvServer;
pub use server::Response;

/// Result type used by this crate
pub type Result<T> = core::result::Result<T, KVError>;

pub fn fibonacci(n: u64) -> u64 {
    match n {
        0 => 1,
        1 => 1,
        n => fibonacci(n-1) + fibonacci(n-2),
    }
}
