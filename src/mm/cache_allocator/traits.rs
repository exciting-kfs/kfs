use core::ptr::NonNull;

use super::meta_cache::MetaCache;
use super::util::dealloc_block_to_page_alloc;
use super::util::no_alloc_list::NAList;

pub trait CacheTrait {
	fn partial(&mut self) -> &mut NAList<MetaCache>;
	fn empty(&self) -> bool;

	fn cache_shrink(&mut self) {
		let m_cache_list = self.partial();

		let (mut satisfied, not) = m_cache_list.iter_mut().partition(|m| m.inuse == 0);
		(*m_cache_list) = not;

		satisfied.iter_mut().for_each(|meta_cache| unsafe {
			let ptr = meta_cache as *mut MetaCache;
			let ptr = NonNull::new_unchecked(ptr.cast());
			dealloc_block_to_page_alloc(ptr)
		});
	}
}

impl PartialEq for dyn CacheTrait {
	fn eq(&self, other: &Self) -> bool {
		self as *const dyn CacheTrait as *const u8 == other as *const dyn CacheTrait as *const u8
	}
}

/// Initialization function for cache allocator.
///
/// # Safety
/// The memory pointed by `ptr` must be reserved for cache allocator.
pub trait CacheInit: Default {
	unsafe fn construct_at<'a>(ptr: NonNull<u8>) -> &'a mut Self {
		let ptr = ptr.as_ptr() as *mut Self;
		(*ptr) = Self::default();
		&mut (*ptr)
	}
}
