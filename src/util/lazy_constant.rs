use core::{cell::UnsafeCell, mem::MaybeUninit, ops::Deref};

#[derive(Debug)]
pub struct LazyConstant<T> {
	value: UnsafeCell<MaybeUninit<T>>,
	initailized: UnsafeCell<bool>,
}

unsafe impl<T> Send for LazyConstant<T> {}
unsafe impl<T> Sync for LazyConstant<T> {}

impl<T> LazyConstant<T> {
	pub const fn uninit() -> Self {
		Self {
			value: UnsafeCell::new(MaybeUninit::uninit()),
			initailized: UnsafeCell::new(false),
		}
	}

	pub unsafe fn as_mut_ptr(&self) -> *mut T {
		self.value.get().as_mut().unwrap().as_mut_ptr()
	}

	pub unsafe fn write(&self, value: T) -> &mut T {
		*self.initailized.get() = true;
		self.value.get().as_mut().unwrap().write(value)
	}
}

impl<T> Deref for LazyConstant<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		unsafe {
			if !*self.initailized.get() {
				panic!("LazyConstant uninitailized!");
			}
			self.value.get().as_ref().unwrap().assume_init_ref()
		}
	}
}
