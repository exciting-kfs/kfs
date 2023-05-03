use crate::mm::util::size_of_rank;

use super::dealloc_block_to_page_alloc;
use super::size_cache::free_list::FreeList;

pub trait CacheBase {
	fn free_list(&mut self) -> &mut FreeList;
	fn page_count(&mut self) -> &mut usize;
	fn rank(&self) -> usize;

	fn cache_shrink(&mut self) {
		let rank = self.rank();
		let free_list = self.free_list();

		let (mut satisfied, not) = free_list
			.iter_mut()
			.partition(|node| node.bytes() >= size_of_rank(rank));
		(*free_list) = not;

		let mut shrinked_page = 0;
		satisfied.iter_mut().for_each(|node| {
			let (blk_ptr, blk_cnt) = node.shrink(free_list, rank);
			shrinked_page += blk_cnt * (1 << rank);
			unsafe { dealloc_block_to_page_alloc(blk_ptr, blk_cnt, rank) };
		});

		(*self.page_count()) -= shrinked_page;
	}
}

impl PartialEq for dyn CacheBase {
	fn eq(&self, other: &Self) -> bool {
		self as *const dyn CacheBase as *const u8 == other as *const dyn CacheBase as *const u8
	}
}

/// Initialization function for cache allocator.
///
/// # Safety
/// The memory pointed by `ptr` must be reserved for cache allocator.
pub trait CacheInit: Default {
	unsafe fn cache_init(ptr: *mut Self) {
		(*ptr) = Self::default();
	}
}

pub const fn align_with_hw_cache(bytes: usize) -> usize {
	const CACHE_LINE_SIZE: usize = 64; // L1

	match bytes {
		0..=16 => 16,
		17..=32 => 32,
		_ => CACHE_LINE_SIZE * ((bytes - 1) / CACHE_LINE_SIZE + 1),
	}
}
