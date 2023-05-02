mod cache;
mod cache_manager;
mod size_cache;

pub use size_cache::{SizeCache, SizeCacheTrait};
pub use cache_manager::CM;

pub const REGISTER_TRY: usize = 3; // TODO config?

use core::ptr::NonNull;
use core::alloc::AllocError;
use core::slice;

use super::PAGE_ALLOC;
use super::GFP;
use super::constant::PAGE_SIZE;

pub fn alloc_block_from_page_alloc<'a>(rank: usize) -> Result<&'a mut [u8], AllocError> {
	unsafe {
		let ptr = PAGE_ALLOC.assume_init_mut().alloc_page(rank, GFP::Normal).map_err(|_| AllocError)?;
		let ptr = ptr.as_ptr() as *mut u8;
		let ptr = slice::from_raw_parts_mut(ptr, PAGE_SIZE * (1 << rank));
		Ok(ptr)
	}
}

/// # Safety
/// 
/// `blk_ptr` must point memory block allocated by `PAGE_ALLOC`
pub unsafe fn dealloc_block_to_page_alloc(blk_ptr: NonNull<u8>, blk_cnt: usize, blk_rank: usize) {
	let size = 1 << blk_rank;

	for index in 0..blk_cnt {
		let ptr = blk_ptr.as_ptr().offset(size * index as isize);
		PAGE_ALLOC.assume_init_mut().free_page(NonNull::new_unchecked(ptr.cast()));
	}
}
