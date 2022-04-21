//! This module contains project's ThreadPool trait
//! and several implementations.

use crate::Result;

/// ThreadPool trait that describes
/// the functionality of a thread pool capable of
/// spawning and managing threads to perform tasks
pub trait ThreadPool: Clone + Send + 'static {
    /// create a new instance
    fn new(capacity: i32) -> Result<Self>;

    /// spawn a new thread
    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F);
}

mod naive;
mod rayon_pool;
mod shared_queue;

pub use naive::NaiveThreadPool;
pub use rayon_pool::RayonThreadPool;
pub use shared_queue::SharedQueueThreadPool;
