use core::{ptr::NonNull};

use crate::BUDDY;

use self::cache::bit_scan_reverse;

pub mod cache;
mod global_allocator;

// use std::alloc::{Layout, Allocator};
// use std::alloc::{alloc, dealloc, System, Global};

pub const PAGE_BITS : usize = 12;
pub const PAGE_SIZE : usize = 1 << PAGE_BITS;
pub const PAGE_NUM_MASK : usize = usize::MAX << PAGE_BITS;

fn rank_of(count: usize) -> usize {
	if count <= 1 {
		0
	} else {
		bit_scan_reverse(count - 1) + 1
	}
}

pub fn alloc_pages_from_buddy<'a>(count: usize) -> Option<&'a mut [u8]> { // tmp

	if count == 0 {
		return None;
	}

	let rank = rank_of(count);
	unsafe {
		let ptr = BUDDY.as_mut().unwrap().alloc_page(rank).ok()?;
		let ptr = ptr.as_ptr() as *mut u8;
		let ptr = core::slice::from_raw_parts_mut(ptr, PAGE_SIZE * count);
		Some(ptr)
	}
}

// unsafe because this function can't validate the memory allocation of a space pointed by ptr.
pub unsafe fn dealloc_pages_to_buddy(ptr: *mut u8, count: usize) {
	if count == 0 || ptr.is_null() {
		return;
	}
	BUDDY.as_mut().unwrap().free_page(NonNull::new_unchecked(ptr.cast()));
}
