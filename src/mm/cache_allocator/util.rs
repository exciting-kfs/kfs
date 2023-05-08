pub mod no_alloc_list;

use crate::mm::constant::PAGE_SIZE;
use crate::mm::{GFP, PAGE_ALLOC};

use core::alloc::AllocError;
use core::ptr::NonNull;

pub const fn align_with_hw_cache(bytes: usize) -> usize {
	const CACHE_LINE_SIZE: usize = 64; // L1

	match bytes {
		0..=16 => 16,
		17..=32 => 32,
		_ => CACHE_LINE_SIZE * ((bytes - 1) / CACHE_LINE_SIZE + 1),
	}
}

pub fn alloc_block_from_page_alloc(rank: usize) -> Result<(NonNull<u8>, usize), AllocError> {
	unsafe {
		let ptr = PAGE_ALLOC
			.assume_init_mut()
			.alloc_page(rank, GFP::Normal)
			.map_err(|_| AllocError)?;
		Ok((ptr.cast(), PAGE_SIZE * (1 << rank)))
	}
}

/// # Safety
///
/// `blk_ptr` must point memory block allocated by `PAGE_ALLOC`
pub unsafe fn dealloc_block_to_page_alloc(blk_ptr: NonNull<u8>) {
	PAGE_ALLOC.assume_init_mut().free_page(blk_ptr.cast());
}
