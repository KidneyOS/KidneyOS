pub mod ticket;
pub use self::ticket::{TicketMutex, TicketMutexGuard};
use core::{
    fmt,
    ops::{Deref, DerefMut},
};

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

    #[inline(always)]
    pub fn try_lock(&self) -> Option<MutexGuard<T>> {
        self.inner
            .try_lock()
            .map(|guard| MutexGuard { inner: guard })
    }

    #[inline(always)]
    pub fn get_mut(&mut self) -> &mut T {
        self.inner.get_mut()
    }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for Mutex<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<T: ?Sized + Default> Default for Mutex<T> {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl<T> From<T> for Mutex<T> {
    fn from(data: T) -> Self {
        Self::new(data)
    }
}

impl<'a, T: ?Sized> MutexGuard<'a, T> {
    #[inline(always)]
    pub fn leak(this: Self) -> &'a mut T {
        InnerMutexGuard::leak(this.inner)
    }
}

impl<'a, T: ?Sized + fmt::Debug> fmt::Debug for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized + fmt::Display> fmt::Display for MutexGuard<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(&**self, f)
    }
}

impl<'a, T: ?Sized> Deref for MutexGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.inner
    }
}

impl<'a, T: ?Sized> DerefMut for MutexGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}
