use criterion::{criterion_group, criterion_main, Criterion};
use kvs_project_4::{thread_pool::*, KvClient, KvServer, KvStore};
use std::thread;
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
        let shutdown_handle = server.terinate_handle();
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
        let mut shutdown = shutdown_handle.lock().unwrap();
        *shutdown = true;
        drop(shutdown);

        // Now our server is blocking so merely toggle the shutdown
        // flag is not enough because the server is probably running a blocking
        // accept method. The workaround is to send a new dummy request to it
        // to let it finish the last accept call and 'gracefully' terminate
        let mut dummy_cli = KvClient::new(SOCK_ADDR).unwrap();
        dummy_cli.sent_get("key".to_owned()).unwrap(); // we don't care if this succeeds or not
        dummy_cli.shutdown().unwrap();
        join_handle.join().unwrap();
    }
}

criterion_group!(group, kv_shared_queue_write);
criterion_main!(group);
