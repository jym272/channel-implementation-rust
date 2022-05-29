use std::collections::VecDeque;
//Basic Concepts for implementing a channel in Rust
// Mutex: https://doc.rust-lang.org/std/sync/struct.Mutex.html
// Condvar: https://doc.rust-lang.org/std/sync/struct.Condvar.html
// Arc: https://doc.rust-lang.org/std/sync/struct.Arc.html
// condvar -> announce to a different threat that I changed something that the other thread is waiting for
// Arc -> thread safe reference counting
// Mutex -> thread safe locking
use std::sync::{Arc, Condvar, Mutex};

struct Inner<T> {
    //Mutex for the sender and the receiver because they are accessing the same data
    //with mutex assures that only one Sender or the Receiver can access the data at a time
    queue: Mutex<VecDeque<T>>,
    available: Condvar,
}
struct Sender<T> {
    //Arc because we want to share the queue between multiple threads,
    //otherwise the sender and the receiver would have two instances of the queue,
    //They need to share the same queue because the sender and the receiver need to be able to send and receive messages
    inner: Arc<Inner<T>>,
}
struct Receiver<T> {
    inner: Arc<Inner<T>>,
}
fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner {
        queue: Mutex::default(), //default is empty vector
    });
    (
        Sender {
            inner: inner.clone(),
        },
        Receiver { inner },
    )
}
impl<T> Sender<T> {
    fn send(&self, t: T) {
        let mut queue = self.inner.queue.lock().unwrap();
        queue.push_back(t);
        drop(queue); //drop the lock
        self.inner.available.notify_one(); //wake up the receiver
    }
}
impl<T> Receiver<T> {
    fn recv(&self) -> T {
        let mut queue = self.inner.queue.lock().unwrap();
        loop {
            match queue.pop_front() {
                Some(t) => return t,
                None => {
                    //you can't wait while still holding the mutex lock.
                    //unwrap -> give up the mutex in order to wait
                    queue = self.inner.available.wait(queue).unwrap();
                }
            }
        }
    }
}
