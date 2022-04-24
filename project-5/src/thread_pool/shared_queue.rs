use super::ThreadPool;
use crate::Result;
use crossbeam::{channel, Receiver, Sender};
use std::thread;
use tracing::debug;

/// ThreadPool Implementation using a shared message queue
#[derive(Clone)]
pub struct SharedQueueThreadPool {
    // thread pool holds onto the sending end
    sender: Sender<Box<dyn FnOnce() + Send + 'static>>,
}

impl ThreadPool for SharedQueueThreadPool {
    fn new(capacity: i32) -> Result<Self> {
        let (tx, rx) = channel::unbounded::<Box<dyn FnOnce() + Send + 'static>>();
        for _ in 0..capacity {
            let rx = TaskReceiver(rx.clone());
            thread::spawn(move || {
                run_task(rx);
            });
        }

        Ok(Self { sender: tx })
    }

    fn spawn<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.sender.send(Box::new(f)).expect("No Threads Available");
    }
}

#[derive(Clone)]
struct TaskReceiver(Receiver<Box<dyn FnOnce() + Send>>);

impl Drop for TaskReceiver {
    fn drop(&mut self) {
        // drop can be called on a TaskReceiver for two reasons
        // we only recover a new thread when the drop is called because
        // of panicking task
        if thread::panicking() {
            let rx = self.0.clone();
            thread::spawn(move || run_task(TaskReceiver(rx)));
        }
    }
}

fn run_task(rx: TaskReceiver) {
    loop {
        match rx.0.recv() {
            Ok(task) => {
                task();
            }

            Err(_err) => {
                debug!("ThreadPool Destroyed, Thread exits");
                break;
            }
        }
    }
}
