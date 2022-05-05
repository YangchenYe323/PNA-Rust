use super::{kv_util::*, KvsEngine};
use crate::thread_pool::ThreadPool;
use crate::{KVError, KVErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tracing::error;

// try to compact log under 2MB threshold
const COMPACTION_THRESHOLD: u64 = 2 * 1024 * 1024;

/// Data Structure handling the storage and retrieval
/// of key-value data
///
/// ```rust
/// use kvs_project_5::{
///     thread_pool::SharedQueueThreadPool,
///     KvStore,
///     KvsEngine
/// };
/// use tempfile::TempDir;
///
/// #[tokio::main]
/// async fn main() {
///
///     let temp_dir = TempDir::new().unwrap();
///     let mut store = KvStore::<SharedQueueThreadPool>::open(temp_dir.path(), 5).unwrap();
///
///     store.set(String::from("key"), String::from("value")).await.unwrap();
///     assert_eq!(Some(String::from("value")), store.get(String::from("key")).await.unwrap());
///
///     store.remove(String::from("key")).await.unwrap();
///     assert_eq!(None, store.get(String::from("key")).await.unwrap());
/// }
///
#[derive(Debug, Clone)]
pub struct KvStore<P: ThreadPool> {
    // referenced by all readers and the writer
    // access of database and cur_read_gen is critical section
    // protected for all kinds of tasks, but it should
    // remain short and in-memory only. Split the disk
    // work through read_half and write_half
    // dirpath: Arc<PathBuf>,
    // database: Arc<Mutex<BTreeMap<String, CommandPos>>>,

    // reader local structures
    read_half: KvStoreReadHalf,

    // writer local structures
    write_half: Arc<Mutex<KvStoreWriteHalf>>,
    pool: P,
}

impl<P: ThreadPool> KvStore<P> {
    /// create a new KvStore instance binded to
    /// given path as its log-file location
    pub fn open(path: impl Into<PathBuf>, capacity: i32) -> Result<Self> {
        let dirpath = Arc::new(path.into());
        // ensure that the log directory exists before proceeding
        fs::create_dir_all(&*dirpath)?;

        let mut database = BTreeMap::new();
        let mut readers = BTreeMap::new();
        let mut uncompacted = 0;
        let gen_list = sorted_gen_list(&dirpath)?;

        for &gen in &gen_list {
            let mut reader = PositionedBufReader::new(File::open(&log_path(&dirpath, gen))?)?;
            let new_uncompacted = load_from_logfile(gen, &mut reader, &mut database)?;
            readers.insert(gen, reader);
            uncompacted += new_uncompacted;
        }

        let cur_gen = gen_list.iter().last().unwrap_or(&0) + 1;
        let writer = new_log_file(&dirpath, cur_gen, &mut readers)?;
        let database = Arc::new(Mutex::new(database));

        // stale gen is initialized to 0 and updated every compaction
        let stale_gen = Arc::new(AtomicU64::new(0));

        let kv_reader = KvStoreReadHalf::new(
            Arc::clone(&dirpath),
            Arc::clone(&database),
            Arc::clone(&stale_gen),
        );

        let kv_writer = KvStoreWriteHalf::new(
            Arc::clone(&dirpath),
            cur_gen,
            Arc::clone(&stale_gen),
            Arc::clone(&database),
            writer,
            uncompacted,
        );

        let pool = P::new(capacity)?;

        Ok(Self {
            // dirpath,
            // database,
            read_half: kv_reader,
            write_half: Arc::new(Mutex::new(kv_writer)),
            pool,
        })
    }
}

#[async_trait::async_trait]
impl<P: ThreadPool> KvsEngine for KvStore<P> {
    async fn get(&self, key: String) -> Result<Option<String>> {
        let (sender, receiver) = oneshot::channel();
        let read_half = self.read_half.clone();

        // we implement asynchrounous on top of synchrounous multi-threading:
        // we start a background thread that does the blocking I/O work and communicate
        // using a channel, which is itself a future. Therefore we can poll the channel's receiving
        // end to know whether it has received result from the working thread
        self.pool.spawn(move || {
            let res = read_half.get(key);
            if sender.send(res).is_err() {
                error!("Receiving End is dropped");
            }
        });

        match receiver.await {
            Ok(r) => r,
            Err(err) => Err(KVError::from(err)),
        }
    }

    async fn set(&self, key: String, val: String) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let write_half = self.write_half.clone();
        self.pool.spawn(move || {
            let res = write_half.lock().unwrap().set(key, val);
            if sender.send(res).is_err() {
                error!("Receiving End is dropped");
            }
        });

        match receiver.await {
            Ok(r) => r,
            Err(err) => Err(KVError::from(err)),
        }
    }

    async fn remove(&self, key: String) -> Result<()> {
        let (sender, receiver) = oneshot::channel();
        let write_half = self.write_half.clone();

        self.pool.spawn(move || {
            let res = write_half.lock().unwrap().remove(key);
            if sender.send(res).is_err() {
                error!("Receiving End is dropped");
            }
        });

        match receiver.await {
            Ok(r) => r,
            Err(err) => Err(KVError::from(err)),
        }
    }
}
#[derive(Debug)]
struct KvStoreReadHalf {
    // the biggest stale generation number
    // readers that reads generation less than this number
    // can be safely dropped
    stale_gen: Arc<AtomicU64>,
    // working directory
    dirpath: Arc<PathBuf>,
    // this is strange... Mutex is needed to make KvStoreReadHalf a Sync Type,
    // which is needed because async functions capture a reference to it.
    // However, we don't really share a KvStoreReadHalf across threads
    readers: Mutex<BTreeMap<u64, PositionedBufReader<File>>>,
    database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
}

impl Clone for KvStoreReadHalf {
    fn clone(&self) -> KvStoreReadHalf {
        KvStoreReadHalf {
            stale_gen: Arc::clone(&self.stale_gen),
            dirpath: Arc::clone(&self.dirpath),
            // readers are not cloned
            readers: Mutex::new(BTreeMap::new()),
            database: Arc::clone(&self.database),
        }
    }
}

impl KvStoreReadHalf {
    fn new(
        dirpath: Arc<PathBuf>,
        database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
        stale_gen: Arc<AtomicU64>,
    ) -> Self {
        Self {
            stale_gen,
            dirpath,
            readers: Mutex::new(BTreeMap::new()),
            database,
        }
    }

    // delete all handles to logfiles with stale generation number
    fn clean_stale_handle(&self) {
        let stale_gen = self.stale_gen.load(Ordering::SeqCst);
        let gens_to_delete: Vec<u64> = self
            .readers
            .lock()
            .unwrap()
            .iter()
            .map(|(key, _)| *key)
            .filter(|&gen| gen <= stale_gen)
            .collect();

        let mut readers = self.readers.lock().unwrap();

        for gen in gens_to_delete {
            readers.remove(&gen);
        }
    }

    fn read_op_at_pos(&self, cmd: CommandPos) -> Result<Ops> {
        self.clean_stale_handle();

        let mut readers = self.readers.lock().unwrap();
        let gen = cmd.gen;
        // we now lazily initialize readers
        // so we will create a reader to a logfile
        // if none exists for now
        // a subtle issue here is: if the current gen we get is stale and
        // corresponding logfile deleted, the File::open will generate an
        // error and get propogated upward, and the user may retry it
        let gen_reader = readers
            .entry(gen)
            .or_insert(PositionedBufReader::new(File::open(log_path(
                &self.dirpath,
                gen,
            ))?)?);

        // read and deserialize Ops
        gen_reader.seek(SeekFrom::Start(cmd.pos))?;
        let entry_reader = gen_reader.take(cmd.len);
        let ops: Ops = serde_json::from_reader(entry_reader)?;
        Ok(ops)
    }

    fn get(&self, key: String) -> Result<Option<String>> {
        // the critical section ends here:
        let cmd = self.database.lock().unwrap().get(&key).copied();

        if let Some(cmd_pos) = cmd {
            let ops = self.read_op_at_pos(cmd_pos)?;
            if let Ops::Set { key: _, val } = ops {
                Ok(Some(val))
            } else {
                Err(KVErrorKind::UnexpectedCommandType.into())
            }
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
struct KvStoreWriteHalf {
    dirpath: Arc<PathBuf>,
    cur_gen: u64,
    // the writer updates stale_gen to let the reader clean
    // stale file handles
    stale_gen: Arc<AtomicU64>,
    writer: PositionedBufWriter<File>,
    database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
    uncompacted: u64,
}

impl KvStoreWriteHalf {
    fn new(
        dirpath: Arc<PathBuf>,
        cur_gen: u64,
        stale_gen: Arc<AtomicU64>,
        database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
        writer: PositionedBufWriter<File>,
        uncompacted: u64,
    ) -> Self {
        Self {
            dirpath,
            cur_gen,
            stale_gen,
            writer,
            database,
            uncompacted,
        }
    }

    fn write_ops(&mut self, op: &Ops) -> Result<CommandPos> {
        // this is the position of the current op
        let pos = self.writer.pos;
        serde_json::to_writer(&mut self.writer, op)?;
        self.writer.flush()?;

        let new_pos = self.writer.pos;
        Ok((self.cur_gen, pos, new_pos - pos).into())
    }

    fn set(&mut self, key: String, val: String) -> Result<()> {
        let op = Ops::set(key, val);
        let cmd_pos = self.write_ops(&op)?;

        if let Ops::Set { key, val: _ } = op {
            if let Some(old_cmd) = self.database.lock().unwrap().insert(key, cmd_pos) {
                self.uncompacted += old_cmd.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        let old_cmd = self.database.lock().unwrap().remove(&key);

        if let Some(old_cmd) = old_cmd {
            self.uncompacted += old_cmd.len;
            let op = Ops::rm(key);
            let _ = self.write_ops(&op)?;

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }

            Ok(())
        } else {
            Err(KVErrorKind::KeyNotFound.into())
        }
    }

    fn compact(&mut self) -> Result<()> {
        self.cur_gen += 1;
        let compaction_gen = self.cur_gen;
        let mut compaction_writer = open_logfile(&self.dirpath, compaction_gen)?;

        let mut readers_cache = BTreeMap::new();

        // copy all the data stored in the in-memory database
        // to a new logfile, this ensures the new logfile contains
        // all the up-to-date data and old logfiles can be deleted
        let mut new_pos: u64 = 0;

        let mut db = self.database.lock().unwrap();
        for cmd_pos in db.values_mut() {
            let reader = readers_cache
                .entry(cmd_pos.gen)
                .or_insert(PositionedBufReader::new(File::open(log_path(
                    &self.dirpath,
                    cmd_pos.gen,
                ))?)?);
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;

            let mut reader = reader.take(cmd_pos.len);
            let length = io::copy(&mut reader, &mut compaction_writer)?;

            // update in-memory database to relfect new log entry
            *cmd_pos = (compaction_gen, new_pos, length).into();
            new_pos += length;
        }
        compaction_writer.flush()?;
        // release the lock,
        // access of database from this point on by readers is safe
        // because all entries now points to the new location
        drop(db);

        // now all the entries in db has been updated, we can update the stale gen
        // to let readers cleanup
        self.stale_gen.store(compaction_gen - 1, Ordering::SeqCst);

        // delete current log files, up to this point
        // these logfiles are replicated and can be safely deleted
        // without risking losing data
        let gens_to_remove: Vec<u64> = readers_cache
            .keys()
            .filter(|&&key| key < compaction_gen)
            .cloned()
            .collect();
        for gen in gens_to_remove {
            let logfile_path = log_path(&self.dirpath, gen);
            fs::remove_file(logfile_path)?;
        }

        self.writer = compaction_writer;
        self.uncompacted = 0;

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(super) enum Ops {
    Set { key: String, val: String },

    Rm { key: String },
}

impl Ops {
    pub(super) fn set(key: String, val: String) -> Self {
        Self::Set { key, val }
    }

    pub(super) fn rm(key: String) -> Self {
        Self::Rm { key }
    }
}

#[derive(Debug)]
pub(super) struct PositionedBufReader<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> PositionedBufReader<R> {
    pub fn new(mut inner: R) -> Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;

        Ok(Self {
            reader: BufReader::new(inner),
            pos,
        })
    }
}

impl<R: Read + Seek> Read for PositionedBufReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let len = self.reader.read(buf)?;
        self.pos += len as u64;
        Ok(len)
    }
}

impl<R: Read + Seek> Seek for PositionedBufReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let new_pos = self.reader.seek(pos)?;
        self.pos = new_pos;
        Ok(new_pos)
    }
}

#[derive(Debug)]
pub(super) struct PositionedBufWriter<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> PositionedBufWriter<W> {
    pub fn new(mut inner: W) -> io::Result<Self> {
        let pos = inner.seek(SeekFrom::Current(0))?;
        Ok(Self {
            writer: BufWriter::new(inner),
            pos,
        })
    }
}

impl<W: Write + Seek> Write for PositionedBufWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.pos += len as u64;
        Ok(len)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

impl<W: Write + Seek> Seek for PositionedBufWriter<W> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.pos = self.writer.seek(pos)?;
        Ok(self.pos)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct CommandPos {
    pub(super) gen: u64,
    pub(super) pos: u64,
    pub(super) len: u64,
}

impl From<(u64, u64, u64)> for CommandPos {
    fn from((gen, pos, len): (u64, u64, u64)) -> Self {
        Self { gen, pos, len }
    }
}
