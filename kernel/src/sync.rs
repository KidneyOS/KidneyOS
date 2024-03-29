use core::arch::asm;
use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

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

static INTR_DISABLE_COUNT: AtomicUsize = AtomicUsize::new(0);

// This function disables interrupt
#[allow(unused)]
pub fn intr_disable() {
    // disable
    unsafe {
        core::arch::asm!("cli", options(nomem, nostack));
    }
}

// This function enables interrupt
pub fn intr_enable() {
    // enable
    unsafe {
        core::arch::asm!("sti", options(nomem, nostack));
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
            "mov {0:e}, [esp]",
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

// A structure for interrupt-based locking that uses a counter to restore interrupt level.
pub struct InterruptCounterLock<T> {
    data: UnsafeCell<T>,
}

// Safety: InterruptLock can be safely sent across threads.
unsafe impl<T> Sync for InterruptCounterLock<T> {}
unsafe impl<T> Send for InterruptCounterLock<T> {}

impl<T> InterruptCounterLock<T> {
    #![allow(unused)]

    // Creates a new InterruptLock.
    pub const fn new(data: T) -> InterruptCounterLock<T> {
        InterruptCounterLock {
            data: UnsafeCell::new(data),
        }
    }

    // Acquires the lock by disabling interrupts and returns a guard.
    // This function would ideally disable interrupts (using assembly or an external function call).
    pub fn lock(&self) -> InterruptCounterLockGuard<T> {
        // Increment the disable count atomically
        INTR_DISABLE_COUNT.fetch_add(1, Ordering::SeqCst);
        intr_disable();
        InterruptCounterLockGuard { lock: self }
    }

    // This function would re-enable interrupts.
    // It's separated for clarity and would be used by the `Drop` implementation of `InterruptLockGuard`.
    fn unlock(&self) {
        // Call function that enables interrupt only when there is no nested interrupt locks.
        // Decrement the disable count atomically and check if it's now zero (fetch_sub returns the previous value before decrement)
        if INTR_DISABLE_COUNT.fetch_sub(1, Ordering::SeqCst) == 1 {
            // enable
            intr_enable();
        }
    }
}

// A guard that provides access to the data protected by the `InterruptLock`.
// When the guard is dropped, the lock is released (interrupts are re-enabled).
pub struct InterruptCounterLockGuard<'a, T> {
    lock: &'a InterruptCounterLock<T>,
}

impl<T> Deref for InterruptCounterLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for InterruptCounterLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for InterruptCounterLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

// A structure for interrupt-based locking by storing the previous interrupt level to restore interrupt level.
pub struct InterruptLock<T> {
    data: UnsafeCell<T>,
    intr_level: AtomicUsize,
}

// Safety: InterruptLock can be safely sent across threads.
unsafe impl<T> Sync for InterruptLock<T> {}
unsafe impl<T> Send for InterruptLock<T> {}

impl<T> InterruptLock<T> {
    #![allow(unused)]
    // Creates a new InterruptLock.
    pub const fn new(data: T) -> InterruptLock<T> {
        InterruptLock {
            data: UnsafeCell::new(data),
            intr_level: AtomicUsize::new(IntrLevel::IntrOn as usize),
        }
    }

    // Acquires the lock by disabling interrupts and returns a guard.
    // This function would ideally disable interrupts (using assembly or an external function call).
    pub fn lock(&self) -> InterruptLockGuard<T> {
        // get previous interrupt level
        let prev_level = intr_get_level();
        intr_disable();
        // store the interrupt level in the lock's field
        self.intr_level.store(prev_level as usize, Ordering::SeqCst);
        InterruptLockGuard { lock: self }
    }

    // This function would re-enable interrupts.
    // It's separated for clarity and would be used by the `Drop` implementation of `InterruptLockGuard`.
    fn unlock(&self) {
        // Call function that enables interrupt only when there is no nested interrupt locks.
        let previous = self.intr_level.load(Ordering::SeqCst);
        let previous_intr_level = match previous {
            0 => IntrLevel::IntrOn,
            1 => IntrLevel::IntrOff,
            _ => panic!("Unexpected value stored in intr_level"),
        };
        if previous_intr_level == IntrLevel::IntrOn {
            intr_enable();
        }
    }
}

// A guard that provides access to the data protected by the `InterruptLock`.
// When the guard is dropped, the lock is released (interrupts are re-enabled).
pub struct InterruptLockGuard<'a, T> {
    lock: &'a InterruptLock<T>,
}

impl<T> Deref for InterruptLockGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for InterruptLockGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for InterruptLockGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}
