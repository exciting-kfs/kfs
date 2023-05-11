use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use super::MemoryAllocator;

pub static mut NORMAL_ALLOC: MemoryAllocator = MemoryAllocator::new();

#[derive(Debug)]
pub struct MemNormal;

unsafe impl Allocator for MemNormal {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		unsafe { NORMAL_ALLOC.allocate(layout) }
	}
	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		NORMAL_ALLOC.deallocate(ptr, layout)
	}
}
