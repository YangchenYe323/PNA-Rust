use crate::Result;
use super::ThreadPool;
use std::thread;


/// Navie ThreadPool
pub struct NaiveThreadPool {

}

impl ThreadPool for NaiveThreadPool {
	type Instance = Self;
	fn new(capacity: i32) -> Result<Self::Instance> {
		Ok(Self {

		})
	}

	fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
		thread::spawn(move || f());
	}
}