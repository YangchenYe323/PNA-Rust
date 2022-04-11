use crate::{KVErrorKind, Result};
use serde::{Deserialize, Serialize};
use serde_json::Deserializer;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

// try to compact log under 2MB threshold
const COMPACTION_THRESHOLD: u64 = 2 * 1024 * 1024;

/// Data Structure handling the storage and retrieval
/// of key-value data
///
/// ```
/// use kvs::KvStore;
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
#[derive(Debug)]
pub struct KvStore {
    dirpath: PathBuf,
    cur_gen: u64,
    readers: BTreeMap<u64, PositionedBufReader<File>>,
    writer: PositionedBufWriter<File>,
    database: BTreeMap<String, CommandPos>,
    uncompacted: u64,
}

#[derive(Serialize, Deserialize, Debug)]
enum Ops {
    Set { key: String, val: String },

    Rm { key: String },
}

impl Ops {
    fn set(key: String, val: String) -> Self {
        Self::Set { key, val }
    }

    fn rm(key: String) -> Self {
        Self::Rm { key }
    }
}

impl KvStore {
    /// create a new KvStore instance binded to
    /// given path as its log-file location
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let dirpath = path.into();
        // ensure that the log directory exists before proceeding
        fs::create_dir_all(&dirpath)?;

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

        Ok(KvStore {
            dirpath,
            cur_gen,
            readers,
            writer,
            database,
            uncompacted,
        })
    }

    /// set key-val pair in the store
    pub fn set(&mut self, key: String, val: String) -> Result<()> {
        let op = Ops::set(key, val);
        // this is the position of this op in the log
        let pos = self.writer.pos;

        // write op to log
        serde_json::to_writer(&mut self.writer, &op)?;
        self.writer.flush()?;

        // update in-memory map between key and CommandPos
        if let Ops::Set { key, .. } = op {
            if let Some(old_op) = self
                .database
                .insert(key, (self.cur_gen, pos, self.writer.pos - pos).into())
            {
                self.uncompacted += old_op.len;

                // handle compaction
                if self.uncompacted > COMPACTION_THRESHOLD {
                    self.compact()?;
                }
            }
        }
        Ok(())
    }

    /// get a copy of owned values associated with key
    /// return None if no values is found
    pub fn get(&mut self, key: String) -> Result<Option<String>> {
        if let Some(cmd_pos) = self.database.get(&key) {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            let reader = reader.take(cmd_pos.len);
            let op: Ops = serde_json::from_reader(reader)?;
            if let Ops::Set { key: _, val } = op {
                Ok(Some(val))
            } else {
                Err(KVErrorKind::UnexpectedCommandType(key).into())
            }
        } else {
            Ok(None)
        }
    }

    /// remove key
    pub fn remove(&mut self, key: String) -> Result<()> {
        if let Some(cmd_pos) = self.database.remove(&key) {
            // old cmd is stale now and count toward
            // compaction-ready logs
            self.uncompacted += cmd_pos.len;

            // append remove entry
            let op = Ops::rm(key);
            serde_json::to_writer(&mut self.writer, &op)?;
            self.writer.flush()?;

            Ok(())
        } else {
            Err(KVErrorKind::KeyNotFound(key).into())
        }
    }

    fn compact(&mut self) -> Result<()> {
        let compaction_gen = self.cur_gen + 1;
        let mut compaction_writer = new_log_file(&self.dirpath, compaction_gen, &mut self.readers)?;
        self.cur_gen += 2;

        // copy all the data stored in the in-memory database
        // to a new logfile, this ensures the new logfile contains
        // all the up-to-date data and old logfiles can be deleted
        let mut new_pos: u64 = 0;

        for cmd_pos in self.database.values_mut() {
            let reader = self
                .readers
                .get_mut(&cmd_pos.gen)
                .expect("Cannot find log reader");
            reader.seek(SeekFrom::Start(cmd_pos.pos))?;
            let mut reader = reader.take(cmd_pos.len);

            let length = io::copy(&mut reader, &mut compaction_writer)?;

            // update in-memory database to relfect new log entry
            *cmd_pos = (compaction_gen, new_pos, length).into();
            new_pos += length;
        }
        compaction_writer.flush()?;

        // delete current log files
        let gens_to_remove: Vec<u64> = self
            .readers
            .keys()
            .filter(|key| **key < compaction_gen)
            .cloned()
            .collect();
        for gen in gens_to_remove {
            let logfile_path = log_path(&self.dirpath, gen);
            self.readers.remove(&gen);
            fs::remove_file(logfile_path)?;
        }

        self.writer = new_log_file(&self.dirpath, self.cur_gen, &mut self.readers)?;
        self.uncompacted = 0;

        Ok(())
    }
}

fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gen_list: Vec<u64> = fs::read_dir(path)?
        .flat_map(|res| -> Result<_> { Ok(res?.path()) })
        .filter(|path| path.is_file() && path.extension() == Some("log".as_ref()))
        .flat_map(|path| {
            path.file_name()
                .and_then(OsStr::to_str)
                .map(|s| s.trim_end_matches(".log"))
                .map(str::parse::<u64>)
        })
        .flatten()
        .collect();

    gen_list.sort_unstable();
    Ok(gen_list)
}

fn log_path(dirpath: &Path, gen: u64) -> PathBuf {
    dirpath.join(format!("{}.log", gen))
}

fn load_from_logfile(
    gen: u64,
    reader: &mut PositionedBufReader<File>,
    database: &mut BTreeMap<String, CommandPos>,
) -> Result<u64> {
    let mut uncompacted = 0;

    let mut pos = reader.seek(SeekFrom::Start(0))?;
    let mut stream = Deserializer::from_reader(reader).into_iter::<Ops>();
    while let Some(op) = stream.next() {
        let new_pos = stream.byte_offset() as u64;
        match op? {
            Ops::Set { key, val: _ } => {
                if let Some(old_op) = database.insert(key, (gen, pos, new_pos - pos).into()) {
                    uncompacted += old_op.len;
                }
            }
            Ops::Rm { key } => {
                if let Some(old_op) = database.remove(&key) {
                    uncompacted += old_op.len;
                }
            }
        }
        pos = new_pos;
    }

    // println!("In-Memory database after startup: {:?}", database);

    Ok(uncompacted)
}

fn new_log_file(
    dirpath: &Path,
    gen: u64,
    readers: &mut BTreeMap<u64, PositionedBufReader<File>>,
) -> Result<PositionedBufWriter<File>> {
    let filepath = log_path(dirpath, gen);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&filepath)?;

    let writer = PositionedBufWriter::new(file)?;

    readers.insert(gen, PositionedBufReader::new(File::open(&filepath)?)?);
    Ok(writer)
}

#[derive(Debug)]
struct PositionedBufReader<R: Read + Seek> {
    reader: BufReader<R>,
    pos: u64,
}

impl<R: Read + Seek> PositionedBufReader<R> {
    fn new(mut inner: R) -> Result<Self> {
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
struct PositionedBufWriter<W: Write + Seek> {
    writer: BufWriter<W>,
    pos: u64,
}

impl<W: Write + Seek> PositionedBufWriter<W> {
    fn new(mut inner: W) -> io::Result<Self> {
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

#[derive(Debug, Copy, Clone)]
struct CommandPos {
    gen: u64,
    pos: u64,
    len: u64,
}

impl From<(u64, u64, u64)> for CommandPos {
    fn from((gen, pos, len): (u64, u64, u64)) -> Self {
        Self { gen, pos, len }
    }
}
