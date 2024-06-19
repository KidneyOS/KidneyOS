pub mod ticket;
pub use self::ticket::{TicketMutex, TicketMutexGuard};

#[cfg(feature = "ticket_mutex")]
type InnerMutex<T> = TicketMutex<T>;
#[cfg(feature = "ticket_mutex")]
type InnerMutexGuard<'a, T> = TicketMutexGuard<'a, T>;

#[cfg(not(feature = "ticket_mutex"))]
type InnerMutex<T> = TicketMutex<T>;
#[cfg(not(feature = "ticket_mutex"))]
type InnerMutexGuard<'a, T> = TicketMutexGuard<'a, T>;

/// A lock that provides mutually exclusive data access.
pub struct Mutex<T: ?Sized> {
    inner: InnerMutex<T>,
}

unsafe impl<T: ?Sized + Send> Sync for Mutex<T> {}
unsafe impl<T: ?Sized + Send> Send for Mutex<T> {}

/// A guard that provides mutable data access.
pub struct MutexGuard<'a, T: 'a + ?Sized> {
    inner: InnerMutexGuard<'a, T>,
}

impl<T> Mutex<T> {
    #[inline(always)]
    pub const fn new(value: T) -> Self {
        Self {
            inner: InnerMutex::new(value),
        }
    }

    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.inner.into_inner()
    }
}

impl<T: ?Sized> Mutex<T> {
    #[inline(always)]
    pub fn lock(&self) -> MutexGuard<T> {
        MutexGuard {
            inner: self.inner.lock(),
        }
    }

    #[inline(always)]
    pub fn is_locked(&self) -> bool {
        self.inner.is_locked()
    }

    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.inner
            .try_lock()
            .map(|guard| MutexGuard { inner: guard })
    }
}
