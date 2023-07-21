mod buddy_allocator;
mod free_list;
mod page_allocator;

use super::Zone;
use core::{alloc::AllocError, ptr::NonNull};
use kfs_macro::context;
use page_allocator::{PageAlloc, PAGE_ALLOC};

#[context(irq_disabled)]
pub fn alloc_pages(rank: usize, zone: Zone) -> Result<NonNull<[u8]>, AllocError> {
	let mut page_alloc = PAGE_ALLOC.lock();
	page_alloc.alloc_pages(rank, zone)
}

#[context(irq_disabled)]
pub fn free_pages(page: NonNull<u8>) {
	let mut page_alloc = PAGE_ALLOC.lock();
	page_alloc.free_pages(page);
}

#[context(irq_disabled)]
pub fn get_available_pages() -> usize {
	let page_alloc = PAGE_ALLOC.lock();
	page_alloc.get_available_pages()
}

pub fn init() {
	unsafe { PageAlloc::init() };
}
