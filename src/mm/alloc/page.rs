mod buddy_allocator;
mod free_list;
mod page_allocator;

use crate::{mm::page::ptr_to_meta, trace_feature};

use super::Zone;
use core::{alloc::AllocError, ptr::NonNull};
use page_allocator::{PageAlloc, PAGE_ALLOC};

/// # CAUTION
///
/// - Zone::High allocation is unmapped, but return a virtual address that is a physical address shifted by VM_OFFSET.
pub fn alloc_pages(rank: usize, zone: Zone) -> Result<NonNull<[u8]>, AllocError> {
	let mut page_alloc = PAGE_ALLOC.lock();

	unsafe { page_alloc.assume_init_mut().alloc_pages(rank, zone) }
}

pub fn free_pages(page: NonNull<u8>) {
	let mut page_alloc = PAGE_ALLOC.lock();

	let mut meta = ptr_to_meta(page); // hmm..

	unsafe {
		let meta = meta.as_mut();
		if meta.inuse() > 1 {
			meta.dec_inuse();
			trace_feature!("mmap_shared", "free_pages: inuse: {}", meta.inuse());
		} else {
			page_alloc.assume_init_mut().free_pages(page);
		}
	}
}

pub fn get_available_pages() -> usize {
	let page_alloc = PAGE_ALLOC.lock();

	unsafe { page_alloc.assume_init_ref().get_available_pages() }
}

pub fn init() {
	unsafe { PageAlloc::init() };
}
