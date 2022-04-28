use super::{kv_util::*, KvsEngine};
use crate::{KVErrorKind, Result, Command, KvServer};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

// try to compact log under 2MB threshold
const COMPACTION_THRESHOLD: u64 = 2 * 1024 * 1024;

/// Data Structure handling the storage and retrieval
/// of key-value data
///
/// ```
/// use kvs_project_4::{KvStore, KvsEngine};
/// use tempfile::TempDir;
///
/// let temp_dir = TempDir::new().unwrap();
/// let mut store = KvStore::open(temp_dir.path()).unwrap();
///
/// store.set(String::from("key"), String::from("value")).unwrap();
/// assert_eq!(Some(String::from("value")), store.get(String::from("key")).unwrap());
///
/// store.remove(String::from("key")).unwrap();
/// assert_eq!(None, store.get(String::from("key")).unwrap());
///
///
///
#[derive(Debug, Clone)]
pub struct KvStore {
    // referenced by all readers and the writer
    // access of database and cur_read_gen is critical section
    // protected for all kinds of tasks, but it should
    // remain short and in-memory only. Split the disk
    // work through read_half and write_half
    dirpath: Arc<PathBuf>,
    database: Arc<Mutex<BTreeMap<String, CommandPos>>>,

    // reader local structures
    read_half: KvStoreReadHalf,

    // writer local structures
    write_half: Arc<Mutex<KvStoreWriteHalf>>,
}

impl KvStore {
    /// create a KvStore instance with working directory specified
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
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
        let min_gen = Arc::new(AtomicU64::new(0));

        let kv_reader = KvStoreReadHalf::new(
            Arc::clone(&dirpath), 
            Arc::clone(&database), 
        );

        let kv_writer = KvStoreWriteHalf::new(
            Arc::clone(&dirpath),
            cur_gen,
            Arc::clone(&database),
            writer,
            uncompacted,
        );

        Ok(Self {
            dirpath,
            database,
            read_half: kv_reader,
            write_half: Arc::new(Mutex::new(kv_writer)),
        })
    }
}

impl KvsEngine for KvStore {
    fn get(&self, key: String) -> Result<Option<String>> {
        self.read_half.get(key)
    }

    fn set(&self, key: String, val: String) -> Result<()> {
        let mut write_half = self.write_half.lock().unwrap();
        write_half.set(key, val)
    }

    fn remove(&self, key: String) -> Result<()> {
        let mut write_half = self.write_half.lock().unwrap();
        write_half.remove(key)
    }
}

#[derive(Debug)]
struct KvStoreReadHalf {
    dirpath: Arc<PathBuf>,
    readers: RefCell<BTreeMap<u64, PositionedBufReader<File>>>,
    database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
}

impl Clone for KvStoreReadHalf {
    fn clone(&self) -> KvStoreReadHalf {
        KvStoreReadHalf {
            dirpath: Arc::clone(&self.dirpath),
            readers: RefCell::new(BTreeMap::new()),
            database: Arc::clone(&self.database),
        }
    }
}

impl KvStoreReadHalf {
    fn new(
        dirpath: Arc<PathBuf>, 
        database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
    ) -> Self {
        Self {
            dirpath,
            readers: RefCell::new(BTreeMap::new()),
            database,
        }        
    }

    fn read_op_at_pos(&self, cmd: CommandPos) -> Result<Ops> {
        let mut readers = self.readers.borrow_mut();
        let gen = cmd.gen;
        // we now lazily initialize readers
        // so we will create a reader to a logfile
        // if none exists for now
        // a subtle issue here is: if the current gen we get is stale and
        // corresponding logfile deleted, the File::open will generate an
        // error and get propogated upward, and the user may retry it
        let gen_reader = readers
            .entry(gen)
            .or_insert(
                PositionedBufReader::new(File::open(log_path(&self.dirpath, gen))?)?
            );
        
        // read and deserialize Ops
        gen_reader.seek(SeekFrom::Start(cmd.pos));
        let mut entry_reader = gen_reader.take(cmd.len);
        let ops: Ops = serde_json::from_reader(entry_reader)?;
        Ok(ops)
    }
    
    fn get(&self, key: String) -> Result<Option<String>> {
        // the critical section ends here:
        let cmd = self.database
            .lock()
            .unwrap()
            .get(&key)
            .map(|cmd| cmd.clone());
        
        if let Some(cmd_pos) = cmd {
            let ops = self.read_op_at_pos(cmd_pos)?;
            if let Ops::Set { key: _, val } = ops {
                Ok(Some(val))
            } else {
                Err(KVErrorKind::UnexpectedCommandType("".to_owned()).into())
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
    writer: PositionedBufWriter<File>,
    database: Arc<Mutex<BTreeMap<String, CommandPos>>>,
    uncompacted: u64,
}

impl KvStoreWriteHalf {
    fn new(
        dirpath: Arc<PathBuf>,
        cur_gen: u64,
        database: Arc<Mutex<BTreeMap<String, CommandPos>>>, 
        writer: PositionedBufWriter<File>,
        uncompacted: u64,
    ) -> Self {
        Self {
            dirpath,
            cur_gen,
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
        let op = Ops::Set { key, val };
        let cmd_pos = self.write_ops(&op)?;

        if let Ops::Set {key, val: _} = op {
            if let Some(old_cmd) = self.
                                                database.
                                                lock().
                                                unwrap().
                                                insert(key, cmd_pos) {
                self.uncompacted += old_cmd.len;
            }
        }

        if self.uncompacted > COMPACTION_THRESHOLD {
            self.compact()?;
        }

        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        let old_cmd = self.
            database.
            lock().
            unwrap().
            remove(&key);

        if let Some(old_cmd) = old_cmd {
            self.uncompacted += old_cmd.len;
            let op = Ops::Rm { key };
            let _ = self.write_ops(&op)?;

            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }

            Ok(())
        } else {
            Err(KVErrorKind::KeyNotFound("Key not found".to_owned()).into())
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
            let reader = readers_cache.
                entry(cmd_pos.gen)
                .or_insert(
                    PositionedBufReader::new(File::open(log_path(&self.dirpath, cmd_pos.gen))?)?
                );
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
