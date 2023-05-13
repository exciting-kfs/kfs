mod cache_manager;
mod meta_cache;
mod size_cache;
mod traits;
mod util;

pub const REGISTER_TRY: usize = 3; // TODO config?

pub use cache_manager::CM;
pub use size_cache::SizeCache;
pub use traits::{CacheStat, CacheTrait};
pub use util::{alloc_block_from_page_alloc, dealloc_block_to_page_alloc};

use core::array::from_fn;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use core::sync::atomic::Ordering;
use core::{alloc::AllocError, sync::atomic::AtomicBool};

use super::memory_allocator::util::{LEVEL_END, LEVEL_MIN};
use crate::new_cache_allocator;

#[derive(Debug)]
pub struct CacheAllocator {
	initialized: AtomicBool,
	cache: [MaybeUninit<NonNull<dyn CacheTrait>>; 6],
}

impl CacheAllocator {
	pub const fn new() -> Self {
		let initialized = AtomicBool::new(false);
		let cache = MaybeUninit::uninit_array::<6>();

		CacheAllocator { initialized, cache }
	}

	pub fn statistic(&mut self) -> CacheAllocatorStat {
		let cache = &mut self.cache;
		let initialized = self.initialized.load(Ordering::Relaxed);

		let cache_stat = match initialized {
			true => from_fn(|i| unsafe { cache[i].assume_init_mut().as_mut().statistic() }),
			false => from_fn(|_| CacheStat::hand_made(0, 0, 0)),
		};

		CacheAllocatorStat {
			initialized,
			cache_stat,
		}
	}

	pub fn allocate(&mut self, level: usize) -> Result<NonNull<[u8]>, AllocError> {
		unsafe { self.lazy_init() };

		match level.checked_sub(LEVEL_END) {
			None => self.get_allocator(level).allocate(),
			Some(_) => panic!("invalid request!"),
		}
	}

	/// # Safety
	///
	/// `ptr` must point memory allocated by `self`.
	pub unsafe fn deallocate(&mut self, ptr: NonNull<u8>, level: usize) {
		self.lazy_init();

		match level.checked_sub(LEVEL_END) {
			None => self.get_allocator(level).deallocate(ptr),
			Some(_) => panic!("invalid request!"),
		}
	}

	unsafe fn lazy_init(&mut self) {
		if !self.initialized.load(Ordering::Relaxed) {
			let cache = &mut self.cache;
			cache[0].write(new_cache_allocator!(64));
			cache[1].write(new_cache_allocator!(128));
			cache[2].write(new_cache_allocator!(256));
			cache[3].write(new_cache_allocator!(512));
			cache[4].write(new_cache_allocator!(1024));
			cache[5].write(new_cache_allocator!(2048));

			self.initialized.store(true, Ordering::Relaxed)
		}
	}

	fn get_allocator(&mut self, level: usize) -> &mut dyn CacheTrait {
		match level {
			6..=11 => unsafe { self.cache[level - LEVEL_MIN].assume_init_mut().as_mut() },
			_ => panic!("invalid level!"),
		}
	}
}

#[macro_export]
macro_rules! new_cache_allocator {
	($size: literal) => {
		CM.new_allocator::<SizeCache<$size>>()
			.expect("out of memory.") // FIXME
	};
}

impl Drop for CacheAllocator {
	fn drop(&mut self) {
		unsafe {
			if self.initialized.load(Ordering::Relaxed) {
				for c in self.cache.iter() {
					CM.drop_allocator(c.assume_init());
				}
			}
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheAllocatorStat {
	initialized: bool,
	cache_stat: [CacheStat; 6],
}

impl CacheAllocatorStat {
	pub const fn hand_made(initialized: bool, cache: [CacheStat; 6]) -> Self {
		Self {
			initialized,
			cache_stat: cache,
		}
	}
}

mod tests {
	use super::*;
	use crate::mm::{
		cache_allocator::{CacheAllocatorStat, CacheStat},
		constant::PAGE_SIZE,
	};
	use kfs_macro::ktest;

	#[ktest]
	fn alloc_dealloc_cache() {
		for level in LEVEL_MIN..LEVEL_END {
			let mut cache = CacheAllocator::new();
			let mut cache_stat = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
			let total = PAGE_SIZE / (1 << level) - 1;
			cache_stat[level - LEVEL_MIN] = CacheStat::hand_made(1, total, 1);

			let ptr = cache.allocate(level);

			assert_eq!(
				cache.statistic(),
				CacheAllocatorStat::hand_made(true, cache_stat)
			);

			// if not dealloc, then panic! will be called.
			unsafe { cache.deallocate(ptr.unwrap().cast(), level) };

			cache_stat[level - LEVEL_MIN] = CacheStat::hand_made(1, total, 0);
			assert_eq!(
				cache.statistic(),
				CacheAllocatorStat::hand_made(true, cache_stat)
			);
		}
	}

	#[ktest]
	fn alloc_twice() {
		let level = 8;
		let mut cache = CacheAllocator::new();
		let mut cache_stat = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
		let total = PAGE_SIZE / (1 << level) - 1;
		cache_stat[level - LEVEL_MIN] = CacheStat::hand_made(1, total, 2);

		let ptr = [cache.allocate(level), cache.allocate(level)];

		assert_eq!(
			cache.statistic(),
			CacheAllocatorStat::hand_made(true, cache_stat)
		);

		// if not dealloc, then panic! will be called.
		unsafe { cache.deallocate(ptr[0].unwrap().cast(), level) };
		unsafe { cache.deallocate(ptr[1].unwrap().cast(), level) };

		cache_stat[level - LEVEL_MIN] = CacheStat::hand_made(1, total, 0);
		assert_eq!(
			cache.statistic(),
			CacheAllocatorStat::hand_made(true, cache_stat)
		);
	}
}
