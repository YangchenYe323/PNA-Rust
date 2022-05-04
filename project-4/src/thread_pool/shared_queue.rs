use super::ThreadPool;
use crate::{KVErrorKind, Result};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use tracing::{error, trace};

trait FnBox {
    fn call_from_box(self: Box<Self>) -> Result<()>;
}

impl<F: FnOnce()> FnBox for F {
    fn call_from_box(self: Box<Self>) -> Result<()> {
        // here we catch panic so that worker can continue running other tasks
        let result = catch_unwind(AssertUnwindSafe(*self));
        if let Err(_error) = result {
            return Err(KVErrorKind::ThreadPanic.into());
        }
        Ok(())
    }
}

type Task = Box<dyn FnBox + Send + 'static>;

enum Message {
    NewTask(Task),
    Terminate,
}

struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let handle = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();
            match message {
                Message::NewTask(task) => {
                    let result = task.call_from_box();
                    if let Err(error) = result {
                        error!("Worker: {}, Error: {}", id, error);
                    }
                }
                Message::Terminate => break,
            }
        });

        Self {
            id,
            handle: Some(handle),
        }
    }
}

/// Shared Queue ThreadPool
/// It maintains a fixed number of workers and send incoming task to
/// workers using a mpsc channel.
///
/// # Note:
/// When dropping a SharedQueueThreadPool, it waits for all its worker threads to terminate,
/// and hence care must be given to not let a worker run an infinite loop. Otherwise the thread pool
/// will also block forever when dropping.
///
/// # Example:
///
/// ```
/// use kvs_project_4::thread_pool::{ThreadPool, SharedQueueThreadPool};
/// use std::sync::{Arc, Mutex};
///
/// let pool = SharedQueueThreadPool::new(5).unwrap();
///
/// let counter: Arc<Mutex<i32>> = Arc::new(Mutex::new(0));
///
/// // increment counter from 5 different threads
/// for _ in 0..5 {
///     let counter = Arc::clone(&counter);
///     pool.spawn(move || {
///         for _ in 0..10 {
///             *(counter.lock().unwrap()) += 1;    
///         }
///     })
/// }
///
/// // dropping the pool will automatically join all its workers
/// drop(pool);
/// assert_eq!(50, *counter.lock().unwrap());
///
pub struct SharedQueueThreadPool {
    num_threads: usize,
    threads: Vec<Worker>,
    // each worker holds a reference to the receiving end
    sender: mpsc::Sender<Message>,
}

impl ThreadPool for SharedQueueThreadPool {
    type Instance = Self;
    fn new(capacity: i32) -> Result<Self::Instance> {
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let num_threads = capacity as usize;
        let mut threads = Vec::with_capacity(num_threads);
        for i in 0..num_threads {
            threads.push(Worker::new(i, receiver.clone()));
        }

        Ok(Self {
            num_threads,
            threads,
            sender,
        })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        let message = Message::NewTask(Box::new(f));
        self.sender.send(message).unwrap();
    }
}

impl Drop for SharedQueueThreadPool {
    fn drop(&mut self) {
        // send terminate signals to all workers
        for _ in 0..self.num_threads {
            self.sender.send(Message::Terminate).unwrap();
        }

        // join workers
        for worker in &mut self.threads {
            trace!("Dropping Worker {}", worker.id);
            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}
