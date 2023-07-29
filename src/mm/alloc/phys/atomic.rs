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
}

unsafe impl Allocator for Atomic {
	#[context(irq_disabled)]
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let mut atomic = ATOMIC_ALLOC.lock();
		atomic.allocate(layout)
	}
	#[context(irq_disabled)]
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let mut atomic = ATOMIC_ALLOC.lock();
		atomic.deallocate(ptr, layout)
	}
}
