use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use kfs_macro::context;

use crate::sync::singleton::Singleton;

use super::PMemAlloc;

pub static ATOMIC_ALLOC: Singleton<PMemAlloc> = Singleton::new(PMemAlloc::uninit());

#[derive(Debug, Clone, Copy, Default)]
pub struct Atomic;

impl Atomic {
	pub fn init() {
		ATOMIC_ALLOC.lock().init();
	}

	#[context(irq_disabled)]
	fn __alloc(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		ATOMIC_ALLOC.lock().allocate(layout)
	}

	#[context(irq_disabled)]
	unsafe fn __dealloc(&self, ptr: NonNull<u8>, layout: Layout) {
		ATOMIC_ALLOC.lock().deallocate(ptr, layout)
	}
}

unsafe impl Allocator for Atomic {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		self.__alloc(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		self.__dealloc(ptr, layout)
	}
}
