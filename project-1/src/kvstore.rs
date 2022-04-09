use std::collections::BTreeMap;

pub struct KvStore {
	map: BTreeMap<String, String>,
}

impl KvStore {
	pub fn new() -> KvStore {
		KvStore { 
			map: BTreeMap::new(),
		}
	}

	pub fn set(&mut self, key: String, val: String) {
		self.map.insert(key, val);
	}

	pub fn get(&self, key: String) -> Option<String> {
		self.map.get(&key).map(|val| {
			val.clone()
		})
	}

	pub fn remove(&mut self, key: String) {
		self.map.remove(&key);
	}
}