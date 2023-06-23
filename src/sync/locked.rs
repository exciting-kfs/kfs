use core::{
	cell::UnsafeCell,
	ops::{Deref, DerefMut},
};

use super::lock::{spinlock::SpinLock, LockType, TryLockFail};

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
		self.inner.lock();
		unsafe { LockedGuard::new(self, LockType::Default) }
	}

	pub fn try_lock(&self) -> Result<LockedGuard<'_, T>, TryLockFail> {
		self.inner
			.try_lock()
			.map(|_| unsafe { LockedGuard::new(self, LockType::Default) })
	}

	pub unsafe fn manual_lock(&self) {
		self.inner.lock();
	}

	pub unsafe fn manual_unlock(&self) {
		self.inner.unlock();
	}

	pub fn lock_irq(&self) -> LockedGuard<'_, T> {
		self.inner.lock();
		unsafe { LockedGuard::new(self, LockType::Irq) }
	}

	pub fn lock_irq_save(&self) -> LockedGuard<'_, T> {
		self.inner.lock();
		unsafe { LockedGuard::new(self, LockType::IrqSave) }
	}
}

pub struct LockedGuard<'lock, T> {
	locked: &'lock Locked<T>,
	kind: LockType,
}

impl<'lock, T> LockedGuard<'lock, T> {
	pub unsafe fn new(locked: &'lock Locked<T>, kind: LockType) -> Self {
		Self { locked, kind }
	}
}

impl<'lock, T> Drop for LockedGuard<'lock, T> {
	fn drop(&mut self) {
		match self.kind {
			LockType::Default => self.locked.inner.unlock(),
			LockType::Irq => self.locked.inner.unlock_irq(),
			LockType::IrqSave => self.locked.inner.unlock_irq_restore(),
		}
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
