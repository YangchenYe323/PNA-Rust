use crate::{KVErrorKind, KvsEngine, Result};
use std::path::Path;

/// Wrapper Around sled database,
pub struct KvSled {
    db: sled::Db,
}

impl KvSled {
    /// open a new KsSled binded with
    /// path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let db = sled::Config::new().path(path).open()?;

        Ok(Self { db })
    }
}

impl KvsEngine for KvSled {
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
            Err(KVErrorKind::KeyNotFound(key).into())
        } else {
            Ok(())
        }
    }
}
