use super::KvsEngine;
use crate::{KVErrorKind, Result};
use std::path::Path;

/// Wrapper Around sled database,
pub struct SledKvsEngine {
    db: sled::Db,
}

impl SledKvsEngine {
    /// open a new instance binded to the given root directory
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::Config::new().path(path).open()?;

        Ok(Self { db })
    }

    /// create a new instance based on given sled database instance
    pub fn new(sled: sled::Db) -> Self {
        Self { db: sled }
    }
}

impl KvsEngine for SledKvsEngine {
    fn get(&mut self, key: String) -> Result<Option<String>> {
        let res = self.db.get(key)?;
        Ok(res.map(|ivec| String::from_utf8(ivec.to_vec()).expect("Utf8 Error")))
    }

    fn set(&mut self, key: String, val: String) -> Result<()> {
        self.db.insert(key, val.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        let res = self.db.remove(key.clone())?;
        self.db.flush()?;
        if res.is_none() {
            Err(KVErrorKind::KeyNotFound.into())
        } else {
            Ok(())
        }
    }
}
