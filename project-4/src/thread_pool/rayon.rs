use super::ThreadPool;
use crate::Result;

/// Rayon ThreadPool
pub struct RayonThreadPool {}

impl ThreadPool for RayonThreadPool {
    type Instance = Self;
    fn new(capacity: i32) -> Result<Self::Instance> {
        unimplemented!()
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        unimplemented!()
    }
}
