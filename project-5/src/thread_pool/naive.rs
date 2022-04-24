use super::ThreadPool;
use crate::Result;
use std::thread;

/// Navie ThreadPool
#[derive(Clone)]
pub struct NaiveThreadPool {}

impl ThreadPool for NaiveThreadPool {
    fn new(_capacity: i32) -> Result<Self> {
        Ok(Self {})
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        thread::spawn(f);
    }
}
