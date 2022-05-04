use criterion::{criterion_group, criterion_main, Criterion};
use kvs_project_4::{thread_pool::*, KvClient, KvServer, KvStore, SledKvsEngine};
use std::{sync::atomic::Ordering, thread};
use tempfile::TempDir;

const SOCK_ADDR: &'static str = "127.0.0.1:4000";
const CONCURRENT_CLIENTS: usize = 10;

// construct clients
// alert: this method doesn't work if server is not up running
// or server blocks for each connection
fn construct_clients(num_clients: usize) -> Vec<KvClient> {
    let mut clients = vec![];
    for _ in 0..num_clients {
        let cli = KvClient::new(SOCK_ADDR).expect("Cannot Connect");
        clients.push(cli);
    }
    clients
}

fn kv_shared_queue_write(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("kv_shared_queue_write");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = SharedQueueThreadPool::new(num_thread).unwrap();
        let store = KvStore::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        group.bench_with_input(
            format!("thread {}", num_thread),
            &num_thread,
            |b, _num_thread| {
                b.iter(move || {
                    // concurrent send request to server
                    let clients = construct_clients(CONCURRENT_CLIENTS);
                    let mut handles = vec![];
                    for mut client in clients {
                        let handle = thread::spawn(move || {
                            for _ in 0..100 {
                                let response = client
                                    .send_set("key".to_owned(), "value".to_owned())
                                    .unwrap();
                                assert!(response.success);
                            }
                            client.shutdown().expect("Cannot Shutdown");
                        });
                        handles.push(handle);
                    }

                    for h in handles {
                        h.join().unwrap();
                    }
                })
            },
        );

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        // Now our server is blocking so merely toggle the shutdown
        // flag is not enough because the server is probably running a blocking
        // accept call. We can switch to non-blocking but that is too much change.
        // The current workaround is to send a new dummy request to it
        // to let it finish the last accept call and 'gracefully' terminate
        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap(); // todo: we actually don't care if this succeeds
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

fn kv_shared_queue_read(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("kv_shared_queue_read");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = SharedQueueThreadPool::new(num_thread).unwrap();
        let store = KvStore::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        // first populate the server with 1000 key-value pairs
        let mut write_cli = KvClient::new(SOCK_ADDR).unwrap();
        for id in 0..100 {
            let response = write_cli
                .send_set(format!("key_{}", id), format!("val_{}", id))
                .unwrap();
            assert!(response.success);
        }
        write_cli.shutdown().unwrap();

        group.bench_function(format!("thread {}", num_thread), |b| {
            b.iter(move || {
                let clients = construct_clients(CONCURRENT_CLIENTS);
                let mut handles = vec![];
                for mut cli in clients {
                    let h = thread::spawn(move || {
                        for id in 0..100 {
                            let response = cli.send_get(format!("key_{}", id)).unwrap();
                            assert!(response.success);
                            assert_eq!(format!("val_{}", id), response.message);
                        }
                        cli.shutdown().unwrap();
                    });
                    handles.push(h);
                }

                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap();
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

fn kv_rayon_write(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("kv_rayon_write");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = RayonThreadPool::new(num_thread).unwrap();
        let store = KvStore::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        group.bench_with_input(
            format!("thread {}", num_thread),
            &num_thread,
            |b, _num_thread| {
                b.iter(move || {
                    // concurrent send request to server
                    let clients = construct_clients(CONCURRENT_CLIENTS);
                    let mut handles = vec![];
                    for mut client in clients {
                        let handle = thread::spawn(move || {
                            for _ in 0..100 {
                                let response = client
                                    .send_set("key".to_owned(), "value".to_owned())
                                    .unwrap();
                                assert!(response.success);
                            }
                            client.shutdown().expect("Cannot Shutdown");
                        });
                        handles.push(handle);
                    }

                    for h in handles {
                        h.join().unwrap();
                    }
                })
            },
        );

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        // Now our server is blocking so merely toggle the shutdown
        // flag is not enough because the server is probably running a blocking
        // accept call. We can switch to non-blocking but that is too much change.
        // The current workaround is to send a new dummy request to it
        // to let it finish the last accept call and 'gracefully' terminate
        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap(); // todo: we actually don't care if this succeeds
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

fn kv_rayon_read(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("kv_rayon_read");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = RayonThreadPool::new(num_thread).unwrap();
        let store = KvStore::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        // first populate the server with 1000 key-value pairs
        let mut write_cli = KvClient::new(SOCK_ADDR).unwrap();
        for id in 0..100 {
            let response = write_cli
                .send_set(format!("key_{}", id), format!("val_{}", id))
                .unwrap();
            assert!(response.success);
        }
        write_cli.shutdown().unwrap();

        group.bench_function(format!("thread {}", num_thread), |b| {
            b.iter(move || {
                let clients = construct_clients(CONCURRENT_CLIENTS);
                let mut handles = vec![];
                for mut cli in clients {
                    let h = thread::spawn(move || {
                        for id in 0..100 {
                            let response = cli.send_get(format!("key_{}", id)).unwrap();
                            assert!(response.success);
                            assert_eq!(format!("val_{}", id), response.message);
                        }
                        cli.shutdown().unwrap();
                    });
                    handles.push(h);
                }

                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap();
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

fn sled_rayon_write(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("sled_rayon_write");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = RayonThreadPool::new(num_thread).unwrap();
        let store = SledKvsEngine::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        group.bench_with_input(
            format!("thread {}", num_thread),
            &num_thread,
            |b, _num_thread| {
                b.iter(move || {
                    // concurrent send request to server
                    let clients = construct_clients(CONCURRENT_CLIENTS);
                    let mut handles = vec![];
                    for mut client in clients {
                        let handle = thread::spawn(move || {
                            for _ in 0..100 {
                                let response = client
                                    .send_set("key".to_owned(), "value".to_owned())
                                    .unwrap();
                                assert!(response.success);
                            }
                            client.shutdown().expect("Cannot Shutdown");
                        });
                        handles.push(handle);
                    }

                    for h in handles {
                        h.join().unwrap();
                    }
                })
            },
        );

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        // Now our server is blocking so merely toggle the shutdown
        // flag is not enough because the server is probably running a blocking
        // accept call. We can switch to non-blocking but that is too much change.
        // The current workaround is to send a new dummy request to it
        // to let it finish the last accept call and 'gracefully' terminate
        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap(); // todo: we actually don't care if this succeeds
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

fn sled_rayon_read(c: &mut Criterion) {
    let threads = [1, 2, 4, 8];
    let mut group = c.benchmark_group("sled_rayon_read");

    for num_thread in threads {
        let dir = TempDir::new().unwrap();
        let pool = RayonThreadPool::new(num_thread).unwrap();
        let store = SledKvsEngine::open(dir.path()).unwrap();
        let server = KvServer::new(SOCK_ADDR, store, pool).unwrap();
        let shutdown = server.terinate_handle();
        // get the server running on the other thread
        let join_handle = thread::spawn(move || {
            server.run();
        });

        // first populate the server with 1000 key-value pairs
        let mut write_cli = KvClient::new(SOCK_ADDR).unwrap();
        for id in 0..100 {
            let response = write_cli
                .send_set(format!("key_{}", id), format!("val_{}", id))
                .unwrap();
            assert!(response.success);
        }
        write_cli.shutdown().unwrap();

        group.bench_function(format!("thread {}", num_thread), |b| {
            b.iter(move || {
                let clients = construct_clients(CONCURRENT_CLIENTS);
                let mut handles = vec![];
                for mut cli in clients {
                    let h = thread::spawn(move || {
                        for id in 0..100 {
                            let response = cli.send_get(format!("key_{}", id)).unwrap();
                            assert!(response.success);
                            assert_eq!(format!("val_{}", id), response.message);
                        }
                        cli.shutdown().unwrap();
                    });
                    handles.push(h);
                }

                for h in handles {
                    h.join().unwrap();
                }
            });
        });

        // shudtown the server
        shutdown.store(true, Ordering::Relaxed);

        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.send_get("key".to_owned()).unwrap();
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

criterion_group!(
    group,
    kv_shared_queue_write,
    kv_rayon_write,
    sled_rayon_write,
    kv_shared_queue_read,
    kv_rayon_read,
    sled_rayon_read,
);
criterion_main!(group);
