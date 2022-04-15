use crate::Result;
use super::ThreadPool;

/// Navie ThreadPool
pub struct NaiveThreadPool {

}

impl ThreadPool for NaiveThreadPool {
	type Instance = Self;
	fn new(capacity: i32) -> Result<Self::Instance> {
		unimplemented!()
	}

	fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
		unimplemented!()
	}
}