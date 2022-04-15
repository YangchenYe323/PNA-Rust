use crate::Result;
use super::ThreadPool;

/// Shared Queue ThreadPool
pub struct SharedQueueThreadPool {

}

impl ThreadPool for SharedQueueThreadPool {
	type Instance = Self;
	fn new(capacity: i32) -> Result<Self::Instance> {
		unimplemented!()
	}

	fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
		unimplemented!()
	}
}