//! A ticket-based mutex based on [spin](https://docs.rs/spin/latest/spin/).

use crate::interrupts::mutex_irq::hold_interrupts;
use crate::interrupts::IntrLevel;
use core::{
    cell::UnsafeCell,
    fmt,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicUsize, Ordering},
};

/// A [spinning mutex](https://en.m.wikipedia.org/wiki/Spinlock) with [ticketing](https://en.wikipedia.org/wiki/Ticket_lock).
///
/// A first-in-first-out ticketing queue: the thread that started waiting first gets the lock first.
///
/// # Example
///
/// ```
/// let lock = sync::mutex::TicketMutex::<_>::new(0);
///
/// *lock.lock() = 1;
/// assert_eq!(*lock.lock(), 1);
/// ```
pub struct TicketMutex<T: ?Sized> {
    next_ticket: AtomicUsize,
    next_serving: AtomicUsize,
    data: UnsafeCell<T>,
}

/// A guard that provides access to the data protected by the mutex.
///
/// When the guard is dropped, the lock is released.
pub struct TicketMutexGuard<'a, T: ?Sized + 'a> {
    next_serving: &'a AtomicUsize,
    ticket: usize,
    data: &'a mut T,
}

// Same unsafe impls as `std::sync::Mutex`
unsafe impl<T: ?Sized + Send> Sync for TicketMutex<T> {}
unsafe impl<T: ?Sized + Send> Send for TicketMutex<T> {}

unsafe impl<T: ?Sized + Sync> Sync for TicketMutexGuard<'_, T> {}
unsafe impl<T: ?Sized + Send> Send for TicketMutexGuard<'_, T> {}

impl<T> TicketMutex<T> {
    #[inline(always)]
    pub const fn new(data: T) -> Self {
        Self {
            next_ticket: AtomicUsize::new(0),
            next_serving: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }

    #[inline(always)]
    pub fn as_mut_ptr(&self) -> *mut T {
        self.data.get()
    }
}

impl<T: ?Sized> TicketMutex<T> {
    #[inline(always)]
    pub fn lock(&self) -> TicketMutexGuard<T> {
        let ticket = self.next_ticket.fetch_add(1, Ordering::Relaxed);

        while self.next_serving.load(Ordering::Acquire) != ticket {
            // We need to yield to something else, otherwise we have to panic!
            let _guard = hold_interrupts(IntrLevel::IntrOn);

            core::hint::spin_loop();
        }

        TicketMutexGuard {
            next_serving: &self.next_serving,
            ticket,
            data: unsafe { &mut *self.data.get() },
        }
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        let ticket = self.next_ticket.load(Ordering::Relaxed);
        self.next_serving.load(Ordering::Relaxed) != ticket
    }

    #[inline(always)]
    pub fn try_lock(&self) -> Option<TicketMutexGuard<T>> {
        let ticket = self
            .next_ticket
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |ticket| {
                if self.next_serving.load(Ordering::Acquire) == ticket {
                    Some(ticket.wrapping_add(1))
                } else {
                    None
                }
            });

        ticket.ok().map(|ticket| TicketMutexGuard {
            next_serving: &self.next_serving,
            ticket,
            data: unsafe { &mut *self.data.get() },
        })
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.get() }
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for TicketMutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.try_lock() {
            Some(guard) => write!(f, "Mutex {{ data: ")
                .and_then(|()| (*guard).fmt(f))
                .and_then(|()| write!(f, "}}")),
            None => write!(f, "Mutex {{ <locked> }}"),
        }
    }
}

impl<T: ?Sized + Default> Default for TicketMutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for TicketMutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized> TicketMutexGuard<'a, T> {
    /// Leak the lock guard, yielding a mutable reference to the underlying data.
    #[inline(always)]
    pub fn leak(this: Self) -> &'a mut T {
        let data = this.data as *mut _; // Keep it in pointer form temporarily to avoid double-aliasing
        core::mem::forget(this);
        unsafe { &mut *data }
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for TicketMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for TicketMutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for TicketMutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.data
    }
}

impl<'a, T: ?Sized> DerefMut for TicketMutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        self.data
    }
}

impl<'a, T: ?Sized> Drop for TicketMutexGuard<'a, T> {
    fn drop(&mut self) {
        let new_ticket = self.ticket + 1;
        self.next_serving.store(new_ticket, Ordering::Release);
    }
}
