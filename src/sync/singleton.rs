use core::{
	cell::UnsafeCell,
	mem::MaybeUninit,
	ops::{Deref, DerefMut},
};

use super::lock::{spinlock::SpinLock, TryLockFail};

#[derive(Debug)]
pub struct Singleton<T> {
	inner: SpinLock,
	value: UnsafeCell<MaybeUninit<T>>,
}

unsafe impl<T> Send for Singleton<T> {}
unsafe impl<T> Sync for Singleton<T> {}

impl<T> Singleton<T> {
	pub const fn uninit() -> Self {
		Self {
			inner: SpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::uninit()),
		}
	}

	pub const fn new(value: T) -> Self {
		Self {
			inner: SpinLock::new(),
			value: UnsafeCell::new(MaybeUninit::new(value)),
		}
	}

	pub unsafe fn as_mut_ptr(&self) -> *mut T {
		self.value.get().as_mut().unwrap().as_mut_ptr()
	}

	pub unsafe fn write(&self, value: T) -> &mut T {
		self.value.get().as_mut().unwrap().write(value)
	}

	pub fn lock(&self) -> SingletonGuard<'_, T> {
		self.inner.lock();
		unsafe { SingletonGuard::new(self) }
	}

	pub fn try_lock(&self) -> Result<SingletonGuard<'_, T>, TryLockFail> {
		self.inner
			.try_lock()
			.map(|_| unsafe { SingletonGuard::new(self) })
	}

	pub unsafe fn lock_manual(&self) -> &mut T {
		self.inner.lock();
		self.value.get().as_mut().unwrap().assume_init_mut()
	}

	pub unsafe fn unlock_manual(&self) {
		self.inner.unlock();
	}
}

pub struct SingletonGuard<'lock, T> {
	singleton: &'lock Singleton<T>,
}

impl<'lock, T> SingletonGuard<'lock, T> {
	pub unsafe fn new(singleton: &'lock Singleton<T>) -> Self {
		Self { singleton }
	}
}

impl<'lock, T> Drop for SingletonGuard<'lock, T> {
	fn drop(&mut self) {
		self.singleton.inner.unlock()
	}
}

impl<'lock, T> Deref for SingletonGuard<'lock, T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe {
			self.singleton
				.value
				.get()
				.as_ref()
				.unwrap()
				.assume_init_ref()
		}
	}
}

impl<'lock, T> DerefMut for SingletonGuard<'lock, T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		unsafe {
			self.singleton
				.value
				.get()
				.as_mut()
				.unwrap()
				.assume_init_mut()
		}
	}
}

// #[cfg(test)]
// mod tests {
// 	static sum: Singleton<usize> = Singleton::new(0);
// 	use super::*;

// 	fn func() {
// 		for _ in 0..100000 {
// 			let gaurd = sum.lock();
// 			// *data += 1;
// 			// (*sum.lock().get_mut()) += 1;
// 			unsafe { *sum.value.get() += 1 };
// 			// drop(gaurd)
// 		}

// 		println!("{}", unsafe { *sum.value.get() });
// 	}

// 	#[test]
// 	fn it_works() {
// 		let mut v: Vec<std::thread::JoinHandle<()>> = vec![];
// 		for _ in 0..3 {
// 			v.push(std::thread::spawn(func));
// 		}

// 		for h in v {
// 			h.join();
// 		}
// 	}
// }
