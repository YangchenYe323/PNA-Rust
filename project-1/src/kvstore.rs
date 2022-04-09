use std::collections::BTreeMap;

/// Data Structure handling the storage and retrieval
/// of key-value data
/// 
/// ```
/// use kvs::KvStore;
/// 
/// let mut store = KvStore::new();
/// store.set(String::from("key"), String::from("value"));
/// assert_eq!(Some(String::from("value")), store.get(String::from("key")));
pub struct KvStore {
	map: BTreeMap<String, String>,
}

impl KvStore {
	/// create a new KvStore instance
	pub fn new() -> KvStore {
		KvStore { 
			map: BTreeMap::new(),
		}
	}

	/// set key-val pair in the store
	pub fn set(&mut self, key: String, val: String) {
		self.map.insert(key, val);
	}

	/// get a copy of owned values associated with key
	/// return None if no values is found
	pub fn get(&self, key: String) -> Option<String> {
		self.map.get(&key).map(|val| {
			val.clone()
		})
	}

	/// remove key
	pub fn remove(&mut self, key: String) {
		self.map.remove(&key);
	}
}