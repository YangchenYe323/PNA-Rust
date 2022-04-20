use super::ThreadPool;
use crate::{Result, KVErrorKind};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::panic::{AssertUnwindSafe, catch_unwind};
use tracing::error;

trait FnBox {
    fn call_from_box(self: Box<Self>) -> Result<()>;
}

impl<F: FnOnce()> FnBox for F {
    fn call_from_box(self: Box<Self>) -> Result<()> {
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
    _id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Self {
        let handle = thread::spawn(move || {
            loop {
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
            } 
        });

        Self {
            _id: id,
            handle: Some(handle),
        }
    }
}

/// Shared Queue ThreadPool
pub struct SharedQueueThreadPool {
    num_threads: usize,
    threads: Vec<Worker>,
    sender: Mutex<mpsc::Sender<Message>>,
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
            sender: Mutex::new(sender),
        })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        let message = Message::NewTask(Box::new(f));
        self.sender.lock().unwrap().send(message).unwrap();
    }
}

impl Drop for SharedQueueThreadPool {
    fn drop(&mut self) {
        for _ in 0..self.num_threads {
            self.sender.lock().unwrap().send(Message::Terminate).unwrap();
        }

        for worker in &mut self.threads {
            if let Some(handle) = worker.handle.take() {
                handle.join().unwrap();
            }
        }
    }
}