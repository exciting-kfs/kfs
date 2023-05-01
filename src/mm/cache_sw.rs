mod cache;
mod cache_manager;
mod size_cache;

pub use size_cache::{SizeCache, ForSizeCache};
pub use cache_manager::CM;

pub const PAGE_BITS : usize = 12;
pub const PAGE_SIZE : usize = 1 << PAGE_BITS;
pub const PAGE_NUM_MASK : usize = usize::MAX << PAGE_BITS;
pub const REGISTER_TRY: usize = 3;

use core::{ptr::NonNull};

use super::{util::bit_scan_reverse, PAGE_ALLOC};
use super::GFP;

fn rank_of(count: usize) -> usize {
	(count > 1).then(|| bit_scan_reverse(count - 1) + 1).unwrap_or_default()
}

pub fn alloc_pages_from_buddy<'a>(count: usize) -> Option<&'a mut [u8]> { // tmp

	if count == 0 {
		return None;
	}

	let rank = rank_of(count);
	unsafe {
		let ptr = PAGE_ALLOC.assume_init_mut().alloc_page(rank, GFP::Normal).ok()?;
		let ptr = ptr.as_ptr() as *mut u8;
		let ptr = core::slice::from_raw_parts_mut(ptr, PAGE_SIZE * (1 << rank));
		Some(ptr)
	}
}

// unsafe because this function can't validate the memory allocation of a space pointed by ptr.
pub unsafe fn dealloc_pages_to_buddy(ptr: *mut u8, count: usize) {
	if count == 0 || ptr.is_null() {
		return;
	}
	PAGE_ALLOC.assume_init_mut().free_page(NonNull::new_unchecked(ptr.cast()));
}
