use core::{
	cell::UnsafeCell,
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

use super::raw_lock::{GlobalSpinLock, TryLockFail};

#[derive(Debug)]
pub struct Locked<T: ?Sized> {
	inner: GlobalSpinLock,
	value: UnsafeCell<T>,
}

unsafe impl<T> Send for Locked<T> {}
unsafe impl<T> Sync for Locked<T> {}

impl<T: Clone> Clone for Locked<T> {
	fn clone(&self) -> Self {
		self.inner.lock();
		let value = UnsafeCell::new(unsafe { (*self.value.get()).clone() });
		self.inner.unlock();
		Self {
			inner: GlobalSpinLock::new(),
			value,
		}
	}
}

impl<T: Default> Default for Locked<T> {
	fn default() -> Self {
		Self {
			inner: GlobalSpinLock::new(),
			value: UnsafeCell::new(T::default()),
		}
	}
}

impl<T> Locked<MaybeUninit<T>> {
	pub const fn uninit() -> Self {
		Self {
			inner: GlobalSpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::uninit()),
		}
	}
}

impl<T, const N: usize> Locked<[MaybeUninit<T>; N]> {
	pub const fn uninit_array() -> Self {
		Self {
			inner: GlobalSpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::uninit_array()),
		}
	}
}

impl<T> Locked<T> {
	pub const fn new(value: T) -> Self {
		Self {
			inner: GlobalSpinLock::new(),
			value: UnsafeCell::new(value),
		}
	}
}

impl<T: ?Sized> Locked<T> {
	pub fn lock(&self) -> LockedGuard<'_, T> {
		self.inner.lock();
		unsafe { LockedGuard::new(self) }
	}

	pub fn try_lock(&self) -> Result<LockedGuard<'_, T>, TryLockFail> {
		self.inner
			.try_lock()
			.map(|_| unsafe { LockedGuard::new(self) })
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

pub struct LockedGuard<'lock, T: ?Sized> {
	locked: &'lock Locked<T>,
}

impl<'lock, T: ?Sized> LockedGuard<'lock, T> {
	pub unsafe fn new(locked: &'lock Locked<T>) -> Self {
		Self { locked }
	}
}

impl<'lock, T: ?Sized> Drop for LockedGuard<'lock, T> {
	fn drop(&mut self) {
		self.locked.inner.unlock();
	}
}

impl<'lock, T: ?Sized> Deref for LockedGuard<'lock, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe { &*self.locked.value.get() }
	}
}

impl<'lock, T: ?Sized> DerefMut for LockedGuard<'lock, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe { &mut *self.locked.value.get() }
	}
}
