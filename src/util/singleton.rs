use core::mem::MaybeUninit;

use super::mutex::spin_mutex::{MutexGuard, SpinMutex};

#[derive(Debug)]
pub struct Singleton<T> {
	data: SpinMutex<MaybeUninit<T>>,
}

impl<T> Singleton<T> {
	pub const fn new(val: T) -> Self {
		Self {
			data: SpinMutex::new(MaybeUninit::new(val)),
		}
	}

	pub const fn uninit() -> Self {
		Self {
			data: SpinMutex::new(MaybeUninit::uninit()),
		}
	}

	pub fn lock<'a>(&'a self) -> SingletonGuard<'a, T> {
		SingletonGuard::new(self)
	}

	pub unsafe fn as_ptr(&self) -> *mut T {
		self.data.as_ptr().cast()
	}

	pub fn write(&self, val: T) {
		self.data.lock().get_mut().write(val);
	}
}

pub struct SingletonGuard<'a, T> {
	lock: MutexGuard<'a, MaybeUninit<T>>,
}

impl<'a, T> SingletonGuard<'a, T> {
	pub fn new(single: &'a Singleton<T>) -> Self {
		let lock = single.data.lock();
		SingletonGuard { lock }
	}

	pub fn get_mut(&mut self) -> &mut T {
		unsafe { self.lock.get_mut().assume_init_mut() }
	}

	pub fn get(&self) -> &T {
		unsafe { self.lock.get().assume_init_ref() }
	}
}

mod tests {

	use super::*;
	use kfs_macro::ktest;

	#[derive(Debug, PartialEq, Eq)]
	struct A(usize);

	impl A {
		unsafe fn construct_at(ptr: *mut A, val: usize) {
			(*ptr).0 = val;
		}
	}

	static S: Singleton<A> = Singleton::uninit();

	#[ktest]
	fn test() {
		unsafe { A::construct_at(S.as_ptr(), 1) };
		assert_eq!(*S.lock().get_mut(), A(1));

		S.lock().get_mut().0 = 2;
		assert_eq!(*S.lock().get_mut(), A(2));
	}
}
