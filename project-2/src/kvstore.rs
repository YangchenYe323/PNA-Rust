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

/// A Persistent Key-Value Storage that uses log-structure file
/// under the hood.
/// 
/// ```
/// use kvs_project_2::KvStore;
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
    // root directory of the KvStore
    dirpath: PathBuf,
    // generation identifier
    // when we compact, we first copy all the existing
    // entries to a new logfile with bigger generation,
    // this ensures that if program crashes during this period,
    // we don't lose any entries we already saved(entries might be duplicated)
    // but that is fine because we'll compact again anyway.
    // After copying is done, we then update the in-memory database to point
    // to the new position in the new logfile. This process if also safe.
    // Finally we delete the stale log files of previous generation, which is totally
    // safe because in-memory database does not refer to them and further write will
    // never be made to them.
    cur_gen: u64,
    // map each alive generation to a reader
    readers: BTreeMap<u64, PositionedBufReader<File>>,
    // we only write to the last generation, so one writer is sufficient
    writer: PositionedBufWriter<File>,
    // in-memory database that maps key to a position in a logfile
    database: BTreeMap<String, CommandPos>,
    // size of uncompacted log entries
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
        // ensure that the root directory exists before proceeding
        fs::create_dir_all(&dirpath)?;

        let mut database = BTreeMap::new();
        let mut readers = BTreeMap::new();
        let mut uncompacted = 0;

        // scan the directory to collect all the existing generation files
        let gen_list = sorted_gen_list(&dirpath)?;

        for &gen in &gen_list {
            // load log entries from generation files to form a in-memory database
            let mut reader = PositionedBufReader::new(File::open(&log_path(&dirpath, gen))?)?;
            let new_uncompacted = load_from_logfile(gen, &mut reader, &mut database)?;
            readers.insert(gen, reader);
            uncompacted += new_uncompacted;
        }

        // start a new generation for writing new log entries
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
        // this is the position of the current op in the log
        let pos = self.writer.pos;

        // write op to log, writer.pos is the end point of the current op
        serde_json::to_writer(&mut self.writer, &op)?;
        self.writer.flush()?;

        // update in-memory map between key and CommandPos
        if let Ops::Set { key, .. } = op {
            // old_op is stale now
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

            // read log entry of cmd_pos
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

            // handle compaction
            if self.uncompacted > COMPACTION_THRESHOLD {
                self.compact()?;
            }

            Ok(())
        } else {
            Err(KVErrorKind::KeyNotFound(key).into())
        }
    }

    // the procedure of compaction is:
    // 1. copy existing key-value entries in the in-memory database to a new logfile
    // 2. update in-memory database to point to new log entries
    // 3. delete old log files
    fn compact(&mut self) -> Result<()> {
        self.cur_gen += 1;
        let compaction_gen = self.cur_gen;
        let mut compaction_writer = new_log_file(&self.dirpath, compaction_gen, &mut self.readers)?;

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

        // delete current log files, up to this point
        // these logfiles are replicated and can be safely deleted
        // without risking losing data
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

        self.writer = compaction_writer;
        self.uncompacted = 0;

        Ok(())
    }
}

// capture all the logfiles in the given directory
// of form "<num>.log"
fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
    let mut gens = vec![];

    let files = fs::read_dir(path)?;

    for entry in files.into_iter() {
        let filename = entry?.path();
        // skip directories and files with other extension
        if filename.is_file() && filename.extension() == Some("log".as_ref()) {
            // println!("{:?}", filename);
            // parse out the generation number, note that parsing error
            // are tolerated and skipped, so "name.log" will not cause
            // the program to crash
            if let Some(name) = filename.file_name() {
                if let Some(name_str) = OsStr::to_str(name) {
                    // println!("{}", name_str);
                    let gen = name_str.trim_end_matches(".log");
                    if let Ok(gen) = gen.parse::<u64>() {
                        // println!("{}", gen);
                        gens.push(gen);
                    }
                }
            }
        }
    }

    gens.sort_unstable();

    // println!("{:?}", gens);

    Ok(gens)
}

// utility function to join directory path and a generation
// number to a path to logfiles
fn log_path(dirpath: &Path, gen: u64) -> PathBuf {
    dirpath.join(format!("{}.log", gen))
}

// read from logfile "<dir>/<gen>.log"
// and update entries in database according to new logs
// book-keep the reader for this file for future use
// returns the number of stale entries in this file
fn load_from_logfile(
    gen: u64,
    reader: &mut PositionedBufReader<File>,
    database: &mut BTreeMap<String, CommandPos>,
) -> Result<u64> {
    // record how many stale logs we have met
    let mut uncompacted = 0;

    let mut pos = reader.seek(SeekFrom::Start(0))?;
    // deserialize the logfile as a sequence of Ops structure
    let mut stream = Deserializer::from_reader(reader).into_iter::<Ops>();
    while let Some(op) = stream.next() {
        // this is the end of the current log and the start of the next
        let new_pos = stream.byte_offset() as u64;
        // replay log
        match op? {
            Ops::Set { key, val: _ } => {
                // old_op is stale now
                if let Some(old_op) = database.insert(key, (gen, pos, new_pos - pos).into()) {
                    uncompacted += old_op.len;
                }
            }
            Ops::Rm { key } => {
                // old_op is stale now
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

// create a new logfile "<dirpath>/gen.log"
// book-keep the readers to logfiles
// and return a writer for the new logfile
fn new_log_file(
    dirpath: &Path,
    gen: u64,
    readers: &mut BTreeMap<u64, PositionedBufReader<File>>,
) -> Result<PositionedBufWriter<File>> {
    let filepath = log_path(dirpath, gen);
    // here we will create a new file clean for modification
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .truncate(true)
        .create(true)
        .open(&filepath)?;

    let writer = PositionedBufWriter::new(file)?;

    readers.insert(gen, PositionedBufReader::new(File::open(&filepath)?)?);
    Ok(writer)
}

// Wrapper around BufReader that is aware of its current offset
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
// Wrapper around BufWriter that is aware of its current offset 
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

// Describes a log on disk
// stored in file "<dirpath>/<gen>.log",
// with offset pos and length len
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
