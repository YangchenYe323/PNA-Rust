use crate::Result;
mod kvsled;
mod kvstore;

pub use kvsled::SledKvsEngine;
pub use kvstore::KvStore;


/// Trait that describe the behavior
/// of a key-value storage engine
pub trait KvsEngine {
    /// get the value of the given string key
    fn get(&mut self, key: String) -> Result<Option<String>>;

    /// set the value of the string key
    fn set(&mut self, key: String, val: String) -> Result<()>;

    /// remove the value of the key
    fn remove(&mut self, key: String) -> Result<()>;
}