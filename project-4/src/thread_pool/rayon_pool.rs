use super::ThreadPool;
use crate::{KVErrorKind, Result};
use failure::ResultExt;
use rayon::{self, ThreadPoolBuilder};

/// Rayon ThreadPool
pub struct RayonThreadPool {
    pool: rayon::ThreadPool,
}

impl ThreadPool for RayonThreadPool {
    type Instance = Self;
    fn new(capacity: i32) -> Result<Self::Instance> {
        let pool = ThreadPoolBuilder::new()
            .num_threads(capacity as usize)
            .build()
            .context(KVErrorKind::RayonError)?;

        Ok(Self { pool })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.pool.spawn(f);
    }
}
