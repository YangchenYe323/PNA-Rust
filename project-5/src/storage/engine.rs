use crate::Result;
use std::{future::Future, pin::Pin};

/// Trait that describe the behavior
/// of a key-value storage engine
pub trait KvsEngine: Clone + Send + 'static {
    /// get the value of the given string key
    fn get(
        &self,
        key: String,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>>;

    /// set the value of the string key
    fn set(
        &self,
        key: String,
        val: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;

    /// remove the value of the key
    fn remove(&self, key: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>>;
}
