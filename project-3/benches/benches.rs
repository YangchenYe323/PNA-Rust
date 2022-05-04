use criterion::{criterion_group, criterion_main, Criterion};
use kvs_project_3::{KvStore, KvsEngine, SledKvsEngine};
use rand::{distributions::Alphanumeric, rngs::SmallRng, Rng, SeedableRng};
use tempfile::TempDir;

// create 100 values
const NUM_VALS: usize = 10;
// this seed is used to determine the size of each key
const KEY_SIZE_SEED: u64 = 233;
// this seed is used for generating random keys
const KEY_SEED: u64 = 757;
// this seed is used to derermine the size of each value
const VALUE_SIZE_SEED: u64 = 2041;
// this seed is used for generating random values
const VALUE_SEED: u64 = 1024;

// this seed is used for generating a sequence of
// index to read
const READ_SEED: u64 = 999;

fn get_size(seed: u64) -> [usize; NUM_VALS] {
    let mut r: SmallRng = SeedableRng::seed_from_u64(seed);
    let mut res = [0; NUM_VALS];
    for i in 0..NUM_VALS {
        res[i] = r.gen_range(1, 100000);
    }
    res
}

fn get_vals(seed: u64, size: &[usize]) -> Vec<String> {
    let mut r: SmallRng = SeedableRng::seed_from_u64(seed);
    let mut res = vec![];
    for s in size {
        let key = r.sample_iter(&Alphanumeric).take(*s).collect();
        res.push(key);
    }
    res
}

/// Benchmark write performance of KvStore
fn bench_write(c: &mut Criterion) {
    // set up keys and values
    let key_sizes = get_size(KEY_SIZE_SEED);
    let keys = get_vals(KEY_SEED, &key_sizes);

    let val_sizes = get_size(VALUE_SIZE_SEED);
    let vals = get_vals(VALUE_SEED, &val_sizes);

    let mut g = c.benchmark_group("bench_write");

    g.bench_function("kvs-write", |b| {
        b.iter(|| {
            // setup
            let tempdir = TempDir::new().unwrap();
            let mut kv = KvStore::open(tempdir.path()).unwrap();

            for i in 0..NUM_VALS {
                let key = keys[i].clone();
                let val = vals[i].clone();
                kv.set(key, val).unwrap();
            }
        })
    });

    g.bench_function("sled-write", |b| {
        b.iter(|| {
            let tempdir = TempDir::new().unwrap();
            let mut sled = SledKvsEngine::open(tempdir.path()).unwrap();

            for i in 0..NUM_VALS {
                let key = keys[i].clone();
                let val = vals[i].clone();
                sled.set(key, val).unwrap();
            }
        })
    });

    g.finish();
}

fn bench_read(c: &mut Criterion) {
    // setup keys and values
    let key_sizes = get_size(KEY_SIZE_SEED);
    let keys = get_vals(KEY_SEED, &key_sizes);

    let val_sizes = get_size(VALUE_SIZE_SEED);
    let vals = get_vals(VALUE_SEED, &val_sizes);

    let kv_tempdir = TempDir::new().unwrap();
    let mut kv = KvStore::open(kv_tempdir.path()).unwrap();

    let sled_tempdir = TempDir::new().unwrap();
    let mut sled = SledKvsEngine::open(sled_tempdir.path()).unwrap();

    // populate the database
    for i in 0..NUM_VALS {
        kv.set(keys[i].clone(), vals[i].clone()).unwrap();
        sled.set(keys[i].clone(), vals[i].clone()).unwrap();
    }

    let mut g = c.benchmark_group("bench_read");

    g.bench_function("kvs-read", |b| {
        b.iter(|| {
            let mut r: SmallRng = SeedableRng::seed_from_u64(READ_SEED);
            // read 1000 times
            for _ in 0..1000 {
                let index = r.gen_range(0, NUM_VALS);
                let key = (&keys[index]).clone();
                assert_eq!(Some(vals[index].clone()), kv.get(key).unwrap());
            }
        })
    });

    g.bench_function("sled-read", |b| {
        b.iter(|| {
            let mut r: SmallRng = SeedableRng::seed_from_u64(READ_SEED);
            // read 1000 times
            for _ in 0..1000 {
                let index = r.gen_range(0, NUM_VALS);
                let key = (&keys[index]).clone();
                assert_eq!(Some(vals[index].clone()), sled.get(key).unwrap());
            }
        })
    });

    g.finish();
}

criterion_group!(group, bench_write, bench_read);
criterion_main!(group);
