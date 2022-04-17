use crate::Result;

/// Trait that describe the behavior
/// of a key-value storage engine
pub trait KvsEngine: Clone + Send + 'static {
    /// get the value of the given string key
    fn get(&self, key: String) -> Result<Option<String>>;

    /// set the value of the string key
    fn set(&self, key: String, val: String) -> Result<()>;

    /// remove the value of the key
    fn remove(&self, key: String) -> Result<()>;
}
