mod address_space;
mod address_tree;
mod test;
mod virtual_allocator;

use core::alloc::{AllocError, Allocator, Layout};
use core::ops::Range;
use core::ptr::NonNull;

use address_space::*;
use address_tree::*;
use virtual_allocator::*;

pub fn vmalloc(layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
	VMALLOC.allocate(layout)
}

pub fn vfree(ptr: NonNull<u8>, layout: Layout) {
	unsafe { VMALLOC.deallocate(ptr, layout) };
}

pub fn vsize(ptr: NonNull<u8>) -> usize {
	VMALLOC.size(ptr)
}

pub fn vinit(area: Range<usize>) {
	VMALLOC.init(area);
}
