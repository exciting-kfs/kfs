use core::alloc::AllocError;
use core::ptr::NonNull;

pub trait CacheTrait: Sync {
	fn empty(&self) -> bool;
	fn size(&self) -> usize;
	fn contains(&mut self, ptr: NonNull<u8>) -> bool;

	fn cache_shrink(&mut self);

	fn stat(&self) -> CacheStat;
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
	/// * The memory pointed by `ptr` must be reserved for cache allocator.
	/// * Because `Self::default` makes a temporal instance, When you implement `drop` trait, be careful.
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
