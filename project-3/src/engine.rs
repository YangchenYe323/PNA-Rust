use crate::Result;

/// Trait that describe the behavior
/// of a key-value storage engine
pub trait KvsEngine {
	/// get the value of the given string key
	fn get(key: String) -> Result<String>;

	/// set the value of the string key
	fn set(key: String, val: String) -> Result<()>;

	/// remove the value of the key
	fn remove(key: String) -> Result<()>;
}