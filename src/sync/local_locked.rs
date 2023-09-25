use core::{
	cell::UnsafeCell,
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

use crate::syscall::errno::Errno;

use super::raw_lock::{LocalSpinLock, TryLockFail};

#[derive(Debug)]
pub struct LocalLocked<T: ?Sized> {
	inner: LocalSpinLock,
	value: UnsafeCell<T>,
}

unsafe impl<T> Send for LocalLocked<T> {}
unsafe impl<T> Sync for LocalLocked<T> {}

impl<T: Clone> Clone for LocalLocked<T> {
	fn clone(&self) -> Self {
		self.inner.lock();
		let value = UnsafeCell::new(unsafe { (*self.value.get()).clone() });
		self.inner.unlock();
		Self {
			inner: LocalSpinLock::new(),
			value,
		}
	}
}

impl<T: Default> Default for LocalLocked<T> {
	fn default() -> Self {
		Self {
			inner: LocalSpinLock::new(),
			value: UnsafeCell::new(T::default()),
		}
	}
}

impl<T> LocalLocked<MaybeUninit<T>> {
	pub const fn uninit() -> Self {
		Self {
			inner: LocalSpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::uninit()),
		}
	}
}

impl<T, const N: usize> LocalLocked<[MaybeUninit<T>; N]> {
	pub const fn uninit_array() -> Self {
		Self {
			inner: LocalSpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::uninit_array()),
		}
	}
}

impl<T> LocalLocked<T> {
	pub const fn new(value: T) -> Self {
		Self {
			inner: LocalSpinLock::new(),
			value: UnsafeCell::new(value),
		}
	}
}

impl<T: ?Sized> LocalLocked<T> {
	pub fn lock(&self) -> LocalLockedGuard<'_, T> {
		self.inner.lock();
		unsafe { LocalLockedGuard::new(self) }
	}

	pub fn lock_check_signal(&self) -> Result<LocalLockedGuard<'_, T>, Errno> {
		self.inner.lock_check_signal()?;
		Ok(unsafe { LocalLockedGuard::new(self) })
	}

	pub fn try_lock(&self) -> Result<LocalLockedGuard<'_, T>, TryLockFail> {
		self.inner
			.try_lock()
			.map(|_| unsafe { LocalLockedGuard::new(self) })
	}

	pub unsafe fn lock_manual(&self) -> &mut T {
		self.inner.lock();
		&mut *self.value.get()
	}

	pub unsafe fn unlock_manual(&self) {
		self.inner.unlock();
	}

	pub unsafe fn get_manual(&self) -> &mut T {
		&mut *self.value.get()
	}
}

pub struct LocalLockedGuard<'lock, T: ?Sized> {
	locked: &'lock LocalLocked<T>,
}

impl<'lock, T: ?Sized> LocalLockedGuard<'lock, T> {
	pub unsafe fn new(locked: &'lock LocalLocked<T>) -> Self {
		Self { locked }
	}
}

impl<'lock, T: ?Sized> Drop for LocalLockedGuard<'lock, T> {
	fn drop(&mut self) {
		self.locked.inner.unlock();
	}
}

impl<'lock, T: ?Sized> Deref for LocalLockedGuard<'lock, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.locked.value.get() }
	}
}

impl<'lock, T: ?Sized> DerefMut for LocalLockedGuard<'lock, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.locked.value.get() }
	}
}
