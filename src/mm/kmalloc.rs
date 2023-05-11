use core::{
	alloc::{AllocError, Allocator, Layout},
	ptr::NonNull,
};

use super::memory_allocator::{mem_atomic::MemAtomic, mem_normal::MemNormal};

pub enum GFP {
	Atomic,
	Normal,
}

pub fn kmalloc(layout: Layout, flag: GFP) -> Result<NonNull<[u8]>, AllocError> {
	match flag {
		GFP::Atomic => MemAtomic.allocate(layout),
		GFP::Normal => MemNormal.allocate(layout),
	}
}

pub unsafe fn kfree(ptr: NonNull<u8>, layout: Layout, flag: GFP) {
	match flag {
		GFP::Atomic => MemAtomic.deallocate(ptr, layout),
		GFP::Normal => MemNormal.deallocate(ptr, layout),
	}
}
