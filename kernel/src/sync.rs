use core::arch::asm;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, Ordering};

// A simple spinlock.
pub struct SpinLock<T> {
    lock: AtomicBool,
    data: UnsafeCell<T>,
}

// Safety: SpinLock can be safely sent across threads.
unsafe impl<T> Sync for SpinLock<T> {}
unsafe impl<T> Send for SpinLock<T> {}

impl<T> SpinLock<T> {
    #![allow(unused)]

    // Creates a new spinlock.
    pub const fn new(data: T) -> SpinLock<T> {
        SpinLock {
            lock: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    // Acquires the spinlock, spinning until the lock is obtained.
    pub fn lock(&self) -> SpinLockGuard<T> {
        while self
            .lock
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            // Spin without locking, to reduce the overhead.
            // This is a simple busy-wait loop.
            while self.lock.load(Ordering::Relaxed) {
                core::hint::spin_loop();
            }
        }
        SpinLockGuard { lock: self }
    }
}

// A guard that provides access to the data protected by the `SpinLock`.
// When the guard is dropped, the lock is released.
pub struct SpinLockGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> Deref for SpinLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for SpinLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for SpinLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.lock.store(false, Ordering::Release);
    }
}

#[allow(unused)]
#[derive(Debug, PartialEq)]
pub enum IntrLevel {
    IntrOn,
    IntrOff,
}

#[allow(unused)]
pub fn intr_get_level() -> IntrLevel {
    let flags: u32;
    unsafe {
        asm!(
            "pushfd",
            "mov {}, [esp]",
            "popfd",
            out(reg) flags
        );
    }

    if flags & (1 << 9) != 0 {
        IntrLevel::IntrOn
    } else {
        IntrLevel::IntrOff
    }
}

// Disables interrupt and returns the previous interrupt level.
#[allow(unused)]
pub fn intr_disable() -> IntrLevel {
    let previous = intr_get_level();
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
    previous
}

// Sets the interrupt level to the given level.
pub fn intr_enable(previous: IntrLevel) {
    if previous == IntrLevel::IntrOn {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }
}
