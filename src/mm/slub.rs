mod cache;
mod cache_manager;
mod size_cache;

pub use size_cache::{SizeCache, ForSizeCache};
pub use cache_manager::CM;

pub const PAGE_BITS : usize = 12;
pub const PAGE_SIZE : usize = 1 << PAGE_BITS;
pub const PAGE_ALIGN : usize = 1 << PAGE_BITS;
pub const PAGE_NUM_MASK : usize = usize::MAX << PAGE_BITS;
pub const REGISTER_TRY: usize = 3;

use core::ptr::NonNull;
use core::alloc::AllocError;

use super::PAGE_ALLOC;
use super::GFP;

pub fn alloc_pages_from_page_alloc<'a>(rank: usize) -> Result<&'a mut [u8], AllocError> {
	unsafe {
		let ptr = PAGE_ALLOC.assume_init_mut().alloc_page(rank, GFP::Normal).map_err(|_| AllocError)?;
		let ptr = ptr.as_ptr() as *mut u8;
		let ptr = core::slice::from_raw_parts_mut(ptr, PAGE_SIZE * (1 << rank));
		Ok(ptr)
	}
}

// unsafe because this function can't validate the memory allocation of a space pointed by ptr.
pub unsafe fn dealloc_pages_to_page_alloc(ptr: *mut u8, count: usize) {
	if ptr.is_null() {
		return;
	}
	PAGE_ALLOC.assume_init_mut().free_page(NonNull::new_unchecked(ptr.cast()));
}
