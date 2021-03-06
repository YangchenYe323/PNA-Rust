//! This module contains project's ThreadPool trait
//! and several implementations.

use crate::Result;

/// ThreadPool trait that describes
/// the functionality of a thread pool capable of
/// spawning and managing threads to perform tasks
pub trait ThreadPool: Send + 'static {
    /// type of the instance
    type Instance: ThreadPool;

    /// create a new instance with given number of threads
    fn new(capacity: i32) -> Result<Self::Instance>;

    /// run the given task using a thread in the pool
    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F);
}

mod naive;
mod rayon_pool;
mod shared_queue;

pub use naive::NaiveThreadPool;
pub use rayon_pool::RayonThreadPool;
pub use shared_queue::SharedQueueThreadPool;
