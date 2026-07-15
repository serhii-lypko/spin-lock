use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;

use crossbeam_utils::CachePadded;

const DEFAULT_THREADS: usize = 12;

fn num_cpu() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(DEFAULT_THREADS)
}

// NOTE: many real-world implementations of mutexes, including std::sync::Mutex on some platforms,
// briefly behave like a spin lock before asking the operating system to put a thread to sleep.
// This is an attempt to combine the best of both worlds.
pub struct MutexedBuffer {
    pub line: Mutex<[u64; 100]>,
}

impl MutexedBuffer {
    pub fn new() -> Self {
        Self {
            line: Mutex::new([0; 100]),
        }
    }
}

pub fn mutexed_buffer() -> [u64; 100] {
    let num_cpu = num_cpu();
    let ready = Arc::new(Barrier::new(num_cpu));

    let buff = Arc::new(MutexedBuffer::new());

    thread::scope(|s| {
        for _ in 0..num_cpu {
            s.spawn({
                let buff = buff.clone();
                let ready = ready.clone();

                move || {
                    ready.wait();

                    for j in 1..10_000 {
                        let mut lock = buff.line.lock().unwrap();
                        lock[j % num_cpu] += 1;
                    }
                }
            });
        }
    });

    *buff.line.lock().unwrap()
}

/* -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- */

// TODO(perf): spin loop issues `swap` (atomic RMW) on every iteration.
// Each RMW demands Exclusive ownership of the cache line (MESI), so N
// spinners force the line to ping-pong between cores, slowing everyone —
// including the holder trying to release.
// Fix: test-and-test-and-set (TTAS) — spin on plain `load(Relaxed)` until
// the flag looks free (loads can share the line in Shared state), only
// then attempt the `swap`.

// Non-optimized: [43.027 ms 43.735 ms 44.414 ms]
pub struct SpinLock<T> {
    locked: CachePadded<AtomicBool>,
    data: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: CachePadded::new(AtomicBool::new(false)),
            data: UnsafeCell::new(value),
        }
    }

    // Returned reference is valid as long as the lock itself exists.
    pub fn lock<'a>(&'a self) -> &'a mut T {
        // swap stores a value into the bool, returning the previous value.
        while self.locked.swap(true, Ordering::Acquire) {
            // Emits a machine instruction to signal the processor that it is running in
            // a busy-wait spin-loop ("spin lock").
            // So the main idea is that spin loop won't emit thread de-scheduling and further
            // context switch. It's CPU-only and has no OS-level communication (as thread::yield_now, which is syscall).
            std::hint::spin_loop();
        }

        // Gets a mutable pointer to the wrapped value.
        unsafe { &mut *self.data.get() }
    }

    pub fn unlock(&self) {
        // Use acquire and release memory ordering to make sure that every unlock()
        // call establishes a happens-before relationship with the lock() calls that follow.
        //
        // In other words, to make sure that after locking it, we can safely assume that
        // whatever happened during the last time it was locked has already happened.
        // This is the most classic use case of acquire and release ordering:
        // acquiring and releasing a lock.
        self.locked.store(false, Ordering::Release);
    }
}

unsafe impl<T: Sized + Send> Send for SpinLock<T> {}
unsafe impl<T: Sized + Send> Sync for SpinLock<T> {}

pub struct SpinLockBuffer<T> {
    pub line: SpinLock<T>,
}

impl<T> SpinLockBuffer<T> {
    pub fn new(value: T) -> Self {
        Self {
            line: SpinLock::new(value),
        }
    }
}

pub fn spin_lock_buffer() -> [u64; 100] {
    let num_cpu = num_cpu();
    let ready = Arc::new(Barrier::new(num_cpu));

    let buff = Arc::new(SpinLockBuffer::new([0u64; 100]));

    thread::scope(|s| {
        for _ in 0..num_cpu {
            s.spawn({
                let buff = buff.clone();
                let ready = ready.clone();

                move || {
                    ready.wait();

                    for j in 1..10_000 {
                        let lock = buff.line.lock();
                        lock[j % num_cpu] += 1;
                        buff.line.unlock();
                    }
                }
            });
        }
    });

    *buff.line.lock()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic() {
        spin_lock_buffer();

        // use std::sync::Barrier;
        // use std::thread;

        // let n = 10;
        // let barrier = Barrier::new(n);
        // thread::scope(|s| {
        //     for _ in 0..n {
        //         // The same messages will be printed together.
        //         // You will NOT see any interleaving.
        //         s.spawn(|| {
        //             println!("before wait");
        //             barrier.wait();
        //             println!("after wait");
        //         });
        //     }
        // });
    }
}
