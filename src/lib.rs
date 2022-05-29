// Mpsc -> Multi Producer Single Consumer
// https://doc.rust-lang.org/std/sync/mpsc/

// Some falvor implementation of mpsc, Crossbeam and Flume
// https://github.com/crossbeam-rs/crossbeam-channel/tree/master/src/flavors
// https://github.com/zesterer/flume

use std::collections::VecDeque;
//Basic Concepts for implementing a channel in Rust
// Mutex: https://doc.rust-lang.org/std/sync/struct.Mutex.html
// Condvar: https://doc.rust-lang.org/std/sync/struct.Condvar.html
// Arc: https://doc.rust-lang.org/std/sync/struct.Arc.html
// condvar -> announce to a different threat that I changed something that the other thread is waiting for
// Arc -> thread safe reference counting
// Mutex -> thread safe locking
use std::sync::{Arc, Condvar, Mutex};

// Flavors:
// 1. Synchronous channels: channel where send() can block, Limited capacity.
        // -> Mutex + Condvar:VecDeque(ring buffer)
        // -> without a Mutex, we can use a atomic queue -> head and tail pointers // wake ups: thread::park + thread::Thread::notify
// 2. Asynchronous channels: channel where send() cannot block, Unbounded.
        // -> Mutex + Condvar:VecDeque
        // -> Mutex + Condvar: LinkedList -> no resizing
        // -> AtomicLinkedList -> linked list of T / Atomic Queue
        // -> Atomic block linked list -> linked list of atomic VecDeque<T>
// 3. Rendezvous channels: Synchronous with capacity 0. Used for thread synchronization.
// 4. Oneshot channels: Any capacity, used for one-time communication. Only one call to send().

// async/await
//send with the channel is full, not blocking, yield to the future


struct Inner<T> {
    queue: VecDeque<T>,
    senders: usize,
}

struct Shared<T> {
    //Mutex for the sender and the receiver because they are accessing the same data
    //with mutex assures that only one Sender or the Receiver can access the data at a time
    inner: Mutex<Inner<T>>,
    available: Condvar,
}

struct Sender<T> {
    //Arc because we want to share the queue between multiple threads,
    //otherwise the sender and the receiver would have two instances of the queue,
    //They need to share the same queue because the sender and the receiver need to be able to send and receive messages
    shared: Arc<Shared<T>>,
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.senders += 1;
        drop(inner);
        Sender {
            shared: Arc::clone(&self.shared), //cloning the arc and not the thing inside the arc
        }
    }
}

impl<T> Drop for Sender<T>{
    fn drop(&mut self) {
        let mut inner = self.shared.inner.lock().unwrap();
        // eprintln!("Sender dropped, count was {}", inner.senders);
        inner.senders -= 1;
        if inner.senders == 0 {
            //only the receiver waiting, at most one thread can be waiting
            self.shared.available.notify_one(); //notify all?
        }
        drop(inner);
    }
}
struct Receiver<T> {
    shared: Arc<Shared<T>>,
    buffer: VecDeque<T>,
}

fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        inner: Mutex::new(Inner {
            queue: VecDeque::new(),
            senders: 1,
        }),
        available: Condvar::new(),
    });
    (
        Sender {
            shared: shared.clone(),

        },
        Receiver {
            shared,
            buffer: VecDeque::new(),
        },
    )
}

impl<T> Sender<T> {
    fn send(&self, t: T) {
        let mut inner = self.shared.inner.lock().unwrap();
        inner.queue.push_back(t); //vector resizing(new allocation, move the values, deAllocation) is not blocking
        drop(inner); //drop the lock
        self.shared.available.notify_one(); //wake up the receiver(the thread in the condvar - a receiver) and immediately take the lock and try to receive the message
    }
}

impl<T> Receiver<T> {
    fn recv(&mut self) -> Option<T> {
        if let Some(t) = self.buffer.pop_front() { //if the buffer is not empty, there is a message in the buffer, we don't have to take the lock
            return Some(t);
        }
        let mut inner = self.shared.inner.lock().unwrap();
        loop {
            match inner.queue.pop_front() {
                Some(t) =>{
                    if !inner.queue.is_empty() {
                        std::mem::swap(&mut self.buffer, &mut inner.queue);
                    }
                    return Some(t)}, //With 'wait' if I'm awake, I take the mutex and check if there is a message, if there is a msg return it, if not I release the mutex and wait again
                None if inner.senders==0   => return None, //if there is no message and there are no senders, return None
                None => { //Block & wait
                    //you can't wait while still holding the mutex lock.
                    //unwrap -> give up the mutex in order to wait
                    inner = self.shared.available.wait(inner).unwrap(); //just before sleep give away the lock/consume the mutex
                    //condvar wait needs a mutex guard
                }
            }
        }
    }
}


impl<T> Iterator for Receiver<T>{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        self.recv()
    }
}


#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn ping_pong() {
        let (tx, mut rx) = channel();
        tx.send(1);
        assert_eq!(rx.recv(), Some(1));
    }

    #[test]
    fn closed_tx() {
        let (tx, mut rx) = channel::<()>();
        // let _ = tx; //this does not drop the sender
        drop(tx);
        // eprintln!("{:?}", "closed test");
        assert_eq!(rx.recv(), None);
        // let _ = rx.recv(); //drop the receiver, but there's no futures senders
    }
    #[test]
     fn closed_rx() {
        let (tx, rx) = channel::<()>();
        drop(rx);
        tx.send(()); //it should return something?
    }
}
