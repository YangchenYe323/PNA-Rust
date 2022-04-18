//! This module contains project's ThreadPool trait
//! and several implementations.

use crate::Result;
use std::panic::UnwindSafe;

/// ThreadPool trait that describes
/// the functionality of a thread pool capable of
/// spawning and managing threads to perform tasks
pub trait ThreadPool {
    /// type of the instance
    type Instance: ThreadPool;

    /// create a new instance
    fn new(capacity: i32) -> Result<Self::Instance>;

    /// spawn a new thread
    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F);
}

mod naive;
mod rayon;
mod shared_queue;

pub use naive::NaiveThreadPool;
pub use rayon::RayonThreadPool;
pub use shared_queue::SharedQueueThreadPool;
