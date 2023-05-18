use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use crate::sync::singleton::Singleton;

use super::MemoryAllocator;

pub static ATOMIC_ALLOC: Singleton<MemoryAllocator> = Singleton::new(MemoryAllocator::uninit());

#[derive(Debug)]
pub struct MemAtomic;

impl MemAtomic {
	pub fn init() {
		ATOMIC_ALLOC.lock().init();
	}
}

unsafe impl Allocator for MemAtomic {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		ATOMIC_ALLOC.lock().allocate(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		ATOMIC_ALLOC.lock().deallocate(ptr, layout)
	}
}
