pub(self) mod kv_util;
mod kvsled;
pub(self) mod kvstore;

pub use kvsled::SledKvsEngine;
pub use kvstore::KvStore;

use crate::Result;
use std::{future::Future, pin::Pin};

/// Trait that describe the behavior
/// of a key-value storage engine
#[async_trait::async_trait]
pub trait KvsEngine: Clone + Send + 'static {
    /// get the value of the given string key
    async fn get(&self, key: String) -> Result<Option<String>>;

    /// set the value of the string key
    async fn set(&self, key: String, val: String) -> Result<()>;

    /// remove the value of the key
    async fn remove(&self, key: String) -> Result<()>;
}
