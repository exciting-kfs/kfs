use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use crate::util::singleton::Singleton;

use super::MemoryAllocator;

pub static ATOMIC_ALLOC: Singleton<MemoryAllocator> = Singleton::new(MemoryAllocator::uninit());

#[derive(Debug)]
pub struct MemAtomic;

impl MemAtomic {
	pub fn init() {
		ATOMIC_ALLOC.lock().get_mut().init();
	}
}

unsafe impl Allocator for MemAtomic {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		ATOMIC_ALLOC.lock().get_mut().allocate(layout)
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		ATOMIC_ALLOC.lock().get_mut().deallocate(ptr, layout)
	}
}
