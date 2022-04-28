# PNA-Rust Project-2 Roadmap

This is a roadmap explaining the challenges in implementing project-2 of PNA in rust.

## KvStore Structure

The first challenge of the project is to find a way to structure logfiles. The first attempt is to use just one logfile. When KvStore is initialized, it opens(or creates) the specified logfile, scans it and populate in-memory databases using logs read from it. When executing `set` and `remove` functions, we also appends to the same logfile. The strcture definition would look loke the following:

```Rust
const LOG_FILE_NAME = "kv.log";
struct KvStore {
  // root directory, logfile: "<dirpath>/<LOG_FILE_NAME>"
	dirpath: PathBuf,
  // reader of logfile
  reader: BufReader<File>
  // writer of logfile
  writer: BufWriter<File>
  // in-memory database
  database: BTreeMap<String, LogEntry>,
}

struct LogEntry {
  // offset into the logfile
  offset: u64,
  // length
  len: u64,
}
```

This structure has two problems:

1. Having all reads and writes executes on the same file **limits the space for future parallelism.** Access to a file through the operating system often requires some form of a read-write lock, which means when a thread is writing to the logfile, no other threads can possibly proceed reading that file.
2. **Compaction is difficult to handle with only one file.** It is not safe to deleting and updating the same file in a series of API calls. Rather, we can use the technique of copying existing entries to a new temporary files and then rename the temporary file to the logfile in an atomic operation, like that used by zookeeper. However, going a step further, **we can just use a series of logfiles identified by a `generation_id`,** so that when compaction is done, we update in memory database to points to LogEntry in the new file, and then safely deletes stale logfiles. Another benefit of this approcah is that write only happens on the logfile with largest generation, and in the process of our program (through shutdown and restart), we might have many logfiles holding log entries, and reading these files can be safely concurrent This point pertains to future projects.

Therefore, we arrived at the following design:

```Rust
struct KvStore {
  // root directory, logfile: "<dirpath>/<LOG_FILE_NAME>"
	dirpath: PathBuf,
  // the biggest generation
  cur_gen: u64,
  // maintain a reader for every alive logfile 
  readers: BTreeMap<u64, BufReader<File>>
  // writer of the current logfile
  writer: BufWriter<File>
  // in-memory database
  database: BTreeMap<String, LogEntry>,
}

struct LogEntry {
  // now a log needs to identify which file it belongs to
  gen: u64,
  offset: u64,
  len: u64,
}
```

Another challenge is that, in order to construct a `LogEntry`, we need to know the offset our writer currently  is at in the file, so that when we append another log, this offset will be the position of it, and the writer's offset after appending will be its end point. To make this possible, we need a wrapper around `BufWriter` as follows:

```Rust
struct PositionedBufWriter<W: Write> {
  inner: BufWriter<W>,
  // keeps track of the current position
  // initialized to 0
  cur_offset: u64,
}
```

And we can easily implements traits `Write` and `Seek` for it to make it easily usable together with libraries like `serde_json`:

```Rust
impl<W: Write> Write for PositionedBufWriter<W> {
  fn write(&mut self, buf: &[u8]) -> Result<usize> {
    let len = self.inner.write(buf)?; // just delegate to the inner writer
    self.cur_offset += len;
  }
}

impl<W: Write> Seek for PositionedBufWriter<W> {
  fn seek(&mut self, seek: SeekFrom) -> Result<u64> {
    let new_offset = self.inner.seek(seek)?;
    self.cu_offset = new_offset;
  }
} 
```

A `PositionedBufReader` can be defined analogously, though it is not really necessary in this project, as the workflow for reading is always: getting a `LogEntry` from im-memory database -> seek the reader by `LogEntry.offset` -> read `LogEntry.len`.

Having these structured defined, it would be easy to implement functions `get, set, remove`

## Compaction Algorithm

The challenge in implementing compaction is to not losing data when program crashes during compaction. Basically, if you delete something from a file and then append it, you risk crashing when you have finished deleting but haven't yet appended. Also, if you have deleted an entry in a logfile, but your thread crashes before you updates the memory database, it might be left with entries that are invalid. (This is not a problem in the current project, but will be an issue when `KvStore` is used in multiple threads).

Given the above, the general procedure of compaction is as follows:

```Rust
fn compact(&mut self) {
  // first copy entries to new logfiles, if crash happens during this
  // stage, nothing is lost, only we will have a partially filled logfile which contains
  // duplicate data. This is OK because we will compact another time anyway.
  copy_data()
  // Then we update the in-memory database BEFORE deleting old logfiles.
  // Crashes during this point is still OK. Nothing is lost, and in-memory database
  // will be updated next time we start
  update_database()
  // Now we can safely deletes the old logfiles, because the database is not pointing
  // to them and their entries have been successfully replicated. 
  delete_old_logs()
  // Now compaction is successful. Database state is consistent throughout the process
}
```