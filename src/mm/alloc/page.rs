mod buddy_allocator;
mod free_list;
mod page_allocator;

use crate::mm::page::get_vmemory_map;

use super::Zone;
use core::{alloc::AllocError, ptr::NonNull};
use page_allocator::{PageAlloc, PAGE_ALLOC};

pub fn alloc_pages(rank: usize, zone: Zone) -> Result<NonNull<[u8]>, AllocError> {
	PAGE_ALLOC.lock().alloc_pages(rank, zone)
}

pub fn free_pages(page: NonNull<u8>) {
	PAGE_ALLOC.lock().free_pages(page);
}

pub fn init() {
	unsafe { PageAlloc::init(&get_vmemory_map()) };
}
