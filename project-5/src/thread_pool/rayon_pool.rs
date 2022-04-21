use super::ThreadPool;
use crate::{Result, KVErrorKind};
use failure::ResultExt;
use rayon;
use std::sync::Arc;

/// Rayon ThreadPool
#[derive(Clone)]
pub struct RayonThreadPool {
    pool: Arc<rayon::ThreadPool>,
}

impl ThreadPool for RayonThreadPool {
    fn new(capacity: i32) -> Result<Self> {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(capacity as usize)
            .build().context(KVErrorKind::RayonError)?;

        Ok(Self {
            pool: Arc::new(pool),
        })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.pool.spawn(f);
    }
}
