use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

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

/// Spawns 100 threads that each bump one slot of a mutex-protected buffer,
/// then returns the final buffer.
pub fn mutexed_buffer() -> [u64; 100] {
    let buff = Arc::new(MutexedBuffer::new());

    thread::scope(|s| {
        for i in 0..100 {
            s.spawn({
                let buff = buff.clone();

                move || {
                    let mut lock = buff.line.lock().unwrap();
                    lock[i] += 1;
                }
            });
        }
    });

    *buff.line.lock().unwrap()
}

/* -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- */

pub struct SpinLock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> SpinLock<T> {
    pub fn new(value: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    // Returned reference is valid as long as the lock itself exists.
    pub fn lock<'a>(&'a self) -> &'a mut T {
        // pub fn lock(&self) -> &mut T {

        // swap stores a value into the bool, returning the previous value.
        while self.locked.swap(true, Ordering::Acquire) {
            // Emits a machine instruction to signal the processor that it is running in
            // a busy-wait spin-loop ("spin lock").
            std::hint::spin_loop();
        }

        // Gets a mutable pointer to the wrapped value.
        unsafe { &mut *self.data.get() }
    }

    pub fn unlock(&self) {
        // Use acquire and release memory ordering to make sure that every unlock()
        // call establishes a happens-before relationship with the lock() calls that follow.
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

/// Spawns 100 threads that each bump one slot of a spin-lock-protected buffer,
/// then returns the final buffer.
pub fn spin_lock_buffer() -> [u64; 100] {
    let buff = Arc::new(SpinLockBuffer::new([0u64; 100]));

    thread::scope(|s| {
        for i in 0..100 {
            s.spawn({
                let buff = buff.clone();

                move || {
                    let lock = buff.line.lock();
                    lock[i] += 1;
                    buff.line.unlock();
                }
            });
        }
    });

    *buff.line.lock()
}
