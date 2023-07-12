use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use kfs_macro::context;

use crate::sync::singleton::Singleton;

use super::PMemAlloc;

pub static NORMAL_ALLOC: Singleton<PMemAlloc> = Singleton::new(PMemAlloc::uninit());

#[derive(Debug)]
pub struct Normal;

impl Normal {
	pub fn init() {
		NORMAL_ALLOC.lock().init();
	}

	#[context(irq_disabled)]
	fn __alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		NORMAL_ALLOC.lock().allocate(layout)
	}

	#[context(irq_disabled)]
	unsafe fn __dealloc(&self, ptr: NonNull<u8>, layout: Layout) {
		NORMAL_ALLOC.lock().deallocate(ptr, layout)
	}
}

unsafe impl Allocator for Normal {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.__alloc(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		self.__dealloc(ptr, layout)
	}
}

mod tests {
	use alloc::vec::Vec;
	use kfs_macro::ktest;

	use super::Normal;

	#[ktest]
	fn with_collection() {
		let mut v = Vec::new_in(Normal);
		for _ in 0..1000000 {
			v.push(1);
		}
	}
}
