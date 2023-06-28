mod address_space;
mod address_tree;
mod test;
mod virtual_allocator;

use core::alloc::{AllocError, Allocator, Layout};
use core::ptr::NonNull;

use address_space::*;
use address_tree::*;
use virtual_allocator::*;

use crate::mm::{constant::*, util::*};

pub fn allocate(layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
	VMALLOC.allocate(layout)
}

pub fn deallocate(ptr: NonNull<u8>, layout: Layout) {
	unsafe { VMALLOC.deallocate(ptr, layout) };
}

pub fn lookup_size(ptr: NonNull<u8>) -> usize {
	VMALLOC.size(ptr)
}

pub fn init() {
	VMALLOC.init(addr_to_pfn(VMALLOC_OFFSET)..LAST_PFN);
}
