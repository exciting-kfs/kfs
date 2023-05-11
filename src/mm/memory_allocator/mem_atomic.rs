use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use super::MemoryAllocator;

pub static mut ATOMIC_ALLOC: MemoryAllocator = MemoryAllocator::new();

#[derive(Debug)]
pub struct MemAtomic;

unsafe impl Allocator for MemAtomic {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		unsafe { ATOMIC_ALLOC.allocate(layout) }
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		ATOMIC_ALLOC.deallocate(ptr, layout)
	}
}
