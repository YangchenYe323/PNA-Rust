use super::kvstore::{CommandPos, Ops, PositionedBufReader, PositionedBufWriter};
use crate::Result;
use serde_json::Deserializer;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

/// scan the given director, find "<num>.log" file
/// and produce a sorted list of such gens
pub(super) fn sorted_gen_list(path: &Path) -> Result<Vec<u64>> {
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

/// util to create "{dirpath}/{gen}.log" as a PargBuf
pub(super) fn log_path(dirpath: &Path, gen: u64) -> PathBuf {
    dirpath.join(format!("{}.log", gen))
}

/// Scan the given gen file from reader, update in-memory
/// database based on entries of the file
pub(super) fn load_from_logfile(
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

/// create a new logfile
pub(super) fn new_log_file(
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

pub(super) fn open_logfile(
    dirpath: &Path,
    gen: u64
) -> Result<PositionedBufWriter<File>> {
    let filepath = log_path(dirpath, gen);
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&filepath)?;
    let writer = PositionedBufWriter::new(file)?;
    Ok(writer)
}
