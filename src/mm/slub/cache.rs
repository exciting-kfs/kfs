use core::ptr::NonNull;

use super::dealloc_block_to_page_alloc;
use super::no_alloc_list::NAList;
use super::size_cache::meta_cache::MetaCache;

pub trait CacheBase {
	fn partial(&mut self) -> &mut NAList<MetaCache>;
	fn empty(&self) -> bool;
	// fn page_count(&mut self) -> &mut usize;
	// fn rank(&self) -> usize;

	fn cache_shrink(&mut self) {
		let m_cache_list = self.partial();

		let (mut satisfied, not) = m_cache_list.iter_mut().partition(|m| m.inuse == 0);
		(*m_cache_list) = not;

		satisfied.iter_mut().for_each(|meta_cache| unsafe {
			let ptr = meta_cache as *mut MetaCache;
			let ptr = NonNull::new_unchecked(ptr.cast());
			dealloc_block_to_page_alloc(ptr, meta_cache.rank())
		});
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
