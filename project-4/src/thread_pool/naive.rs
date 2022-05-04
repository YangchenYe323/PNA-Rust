use super::ThreadPool;
use crate::Result;
use std::thread;

/// Navie ThreadPool just opens a new thread for each task it is given
pub struct NaiveThreadPool {}

impl ThreadPool for NaiveThreadPool {
    type Instance = Self;
    fn new(_capacity: i32) -> Result<Self::Instance> {
        Ok(Self {})
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        thread::spawn(f);
    }
}
