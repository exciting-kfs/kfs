use core::{
	cell::UnsafeCell,
	ops::{Deref, DerefMut},
};

use super::lock::{spinlock::SpinLock, TryLockFail};

#[derive(Debug)]
pub struct Locked<T> {
	inner: SpinLock,
	value: UnsafeCell<T>,
}

unsafe impl<T> Send for Locked<T> {}
unsafe impl<T> Sync for Locked<T> {}

impl<T> Locked<T> {
	pub const fn new(value: T) -> Self {
		Self {
			inner: SpinLock::new(),
			value: UnsafeCell::new(value),
		}
	}

	pub unsafe fn as_mut_ptr(&self) -> *mut T {
		self.value.get()
	}

	pub fn lock(&self) -> LockedGuard<'_, T> {
		self.inner.lock_irq_save();
		unsafe { LockedGuard::new(self) }
	}

	pub fn try_lock(&self) -> Result<LockedGuard<'_, T>, TryLockFail> {
		self.inner
			.try_lock()
			.map(|_| unsafe { LockedGuard::new(self) })
	}
}

pub struct LockedGuard<'lock, T> {
	locked: &'lock Locked<T>,
}

impl<'lock, T> LockedGuard<'lock, T> {
	pub unsafe fn new(locked: &'lock Locked<T>) -> Self {
		Self { locked }
	}
}

impl<'lock, T> Drop for LockedGuard<'lock, T> {
	fn drop(&mut self) {
		self.locked.inner.unlock_irq_restore()
	}
}

impl<'lock, T> Deref for LockedGuard<'lock, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { self.locked.value.get().as_ref().unwrap() }
	}
}

impl<'lock, T> DerefMut for LockedGuard<'lock, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { self.locked.value.get().as_mut().unwrap() }
	}
}
