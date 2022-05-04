use super::KvsEngine;
use crate::Result;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;

/// Wrapper Around sled database,
#[derive(Clone)]
pub struct SledKvsEngine {
    _db: sled::Db,
}

impl SledKvsEngine {
    /// open a new instance binded with
    /// path
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let _db = sled::Config::new().path(path).open()?;

        Ok(Self { _db })
    }

    /// create a new instance based on given sled database instance
    pub fn new(sled: sled::Db) -> Self {
        Self { _db: sled }
    }
}

impl KvsEngine for SledKvsEngine {
    fn get(
        &self,
        _key: String,
    ) -> Pin<Box<dyn Future<Output = Result<Option<String>>> + Send + 'static>> {
        // let res = self.db.get(key)?;
        // Ok(res.map(|ivec| String::from_utf8(ivec.to_vec()).expect("Utf8 Error")))
        unimplemented!()
    }

    fn set(
        &self,
        _key: String,
        _val: String,
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
        // self.db.insert(key, val.as_bytes())?;
        // self.db.flush()?;
        // Ok(())
        unimplemented!()
    }

    fn remove(&self, _key: String) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'static>> {
        // let res = self.db.remove(key.clone())?;
        // self.db.flush()?;
        // if res.is_none() {
        //     Err(KVErrorKind::KeyNotFound(key).into())
        // } else {
        //     Ok(())
        // }
        unimplemented!()
    }
}
