use crate::interrupts::{intr_disable, intr_enable};
use crate::sync::mutex::{Mutex, MutexGuard};
use core::{
    fmt,
    ops::{Deref, DerefMut},
};
use core::sync::atomic::{AtomicU32, Ordering};

/// A guard for withholding interrupts.
#[derive(Default)]
pub struct InterruptsGuard;

impl !Send for InterruptsGuard {}

static INTERRUPT_DISABLE_COUNT: AtomicU32 = AtomicU32::new(0);

pub fn interrupt_increment() { // disable
    INTERRUPT_DISABLE_COUNT.fetch_add(1, Ordering::SeqCst);

    intr_disable();
}

pub fn interrupt_decrement() { // enable
    INTERRUPT_DISABLE_COUNT.fetch_sub(1, Ordering::SeqCst);

    intr_enable();
}

/// Prevents interrupts from occurring until the `InterruptsGuard` is dropped.
/// After it is dropped, the interrupts are returned to the previous state.
pub fn hold_interrupts() -> InterruptsGuard {
    interrupt_increment();

    InterruptsGuard
}

impl Drop for InterruptsGuard {
    fn drop(&mut self) {
        interrupt_decrement()
    }
}

pub struct MutexIrq<T: ?Sized> {
    lock: Mutex<T>,
}

pub struct MutexGuardIrq<'a, T: ?Sized + 'a> {
    guard: MutexGuard<'a, T>,
    _guard: InterruptsGuard,
}

// Same unsafe impls as `std::sync::MutexIrqSafe`
unsafe impl<T: ?Sized + Send> Sync for MutexIrq<T> {}
unsafe impl<T: ?Sized + Send> Send for MutexIrq<T> {}

#[allow(unused)]
impl<T> MutexIrq<T> {
    pub const fn new(data: T) -> MutexIrq<T> {
        MutexIrq {
            lock: Mutex::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.lock.into_inner()
    }
}

#[allow(unused)]
impl<T: ?Sized> MutexIrq<T> {
    #[inline(always)]
    pub fn lock(&self) -> MutexGuardIrq<T> {
        loop {
            if let Some(guard) = self.try_lock() {
                return guard;
            }
        }
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.lock.is_locked()
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<MutexGuardIrq<T>> {
        if self.lock.is_locked() {
            return None;
        }
        let _held_irq = hold_interrupts();
        self.lock.try_lock().map(|guard| MutexGuardIrq {
            guard,
            _guard: _held_irq,
        })
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.lock.get_mut()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for MutexIrq<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.lock.try_lock() {
            Some(guard) => write!(f, "MutexIrq {{ data: {:?} }}", &*guard),
            None => write!(f, "MutexIrq {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for MutexIrq<T> {
    fn default() -> MutexIrq<T> {
        MutexIrq::new(Default::default())
    }
}

impl<'a, T: ?Sized> Deref for MutexGuardIrq<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        &(self.guard)
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuardIrq<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.guard
    }
}
