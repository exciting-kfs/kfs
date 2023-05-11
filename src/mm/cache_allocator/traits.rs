use core::alloc::AllocError;
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

	fn statistic(&self) -> CacheStat;
	fn allocate(&mut self) -> Result<NonNull<[u8]>, AllocError>;
	unsafe fn deallocate(&mut self, ptr: NonNull<u8>);
}

impl PartialEq for dyn CacheTrait {
	fn eq(&self, other: &Self) -> bool {
		self as *const dyn CacheTrait as *const u8 == other as *const dyn CacheTrait as *const u8
	}
}

pub trait CacheInit: Default {
	/// Initialization function for cache allocator.
	///
	/// # Safety
	/// The memory pointed by `ptr` must be reserved for cache allocator.
	unsafe fn construct_at<'a>(ptr: NonNull<u8>) -> &'a mut Self {
		let ptr = ptr.as_ptr() as *mut Self;
		(*ptr) = Self::default();
		&mut (*ptr)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheStat {
	pub page_count: usize,
	pub total: usize,
	pub inuse: usize,
}

impl CacheStat {
	pub const fn hand_made(page_count: usize, total: usize, inuse: usize) -> Self {
		Self {
			page_count,
			total,
			inuse,
		}
	}
}
