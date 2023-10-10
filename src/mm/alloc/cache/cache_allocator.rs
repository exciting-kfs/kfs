use super::cache_manager::CM;
use super::size_cache::SizeCache;
use super::traits::{CacheStat, CacheTrait};

use core::array::from_fn;
use core::mem::MaybeUninit;
use core::ptr::NonNull;

use crate::mm::constant::*;
use crate::sync::Locked;

macro_rules! new_cache_allocator {
	($size:literal) => {
		CM.new_allocator::<Locked<SizeCache<$size>>>()
			.expect("out of memory.")
	};
}
use new_cache_allocator;

#[derive(Debug)]
pub struct CacheAllocator {
	cache: [MaybeUninit<NonNull<dyn CacheTrait>>; NR_CACHE_ALLOCATOR],
}

impl CacheAllocator {
	pub const fn uninit() -> Self {
		let cache = MaybeUninit::uninit_array::<NR_CACHE_ALLOCATOR>();

		CacheAllocator { cache }
	}

	pub fn initialized() -> Self {
		let mut allocator = CacheAllocator::uninit();
		allocator.init();
		allocator
	}

	pub fn init(&mut self) {
		unsafe {
			let cache = &mut self.cache;
			cache[0].write(new_cache_allocator!(64));
			cache[1].write(new_cache_allocator!(128));
			cache[2].write(new_cache_allocator!(256));
			cache[3].write(new_cache_allocator!(512));
			cache[4].write(new_cache_allocator!(1024));
			cache[5].write(new_cache_allocator!(2048));
		}
	}

	pub fn stat(&mut self) -> CacheAllocatorStat {
		let cache = &mut self.cache;
		let cache_stat = from_fn(|i| unsafe { cache[i].assume_init_mut().as_mut().stat() });

		CacheAllocatorStat { cache_stat }
	}

	#[inline]
	pub fn get(&mut self, index: usize) -> &mut dyn CacheTrait {
		unsafe { self.cache[index].assume_init_mut().as_mut() }
	}
}

impl Drop for CacheAllocator {
	fn drop(&mut self) {
		for c in self.cache.iter() {
			unsafe { CM.drop_allocator(c.assume_init()) };
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheAllocatorStat {
	cache_stat: [CacheStat; 6],
}

impl CacheAllocatorStat {
	pub const fn hand_made(cache: [CacheStat; 6]) -> Self {
		Self { cache_stat: cache }
	}
}

mod tests {
	use core::cmp::max;

	use super::*;
	use kfs_macro::ktest;

	#[ktest(cache_allocator)]
	fn alloc_dealloc_cache() {
		for index in 0..NR_CACHE_ALLOCATOR {
			let mut cache = CacheAllocator::initialized();
			let mut cache_stat = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));

			let (ptr, total, count) = {
				let allocator = cache.get(index);

				let total = max(7, PAGE_SIZE / allocator.size() - 1);
				let count = max(allocator.size() * 7, PAGE_SIZE).next_power_of_two() / PAGE_SIZE;
				(allocator.allocate(), total, count)
			};

			cache_stat[index] = CacheStat::hand_made(count, total, 1);
			assert_eq!(cache.stat(), CacheAllocatorStat::hand_made(cache_stat));

			// if not dealloc, then panic! will be called.
			unsafe { cache.get(index).deallocate(ptr.unwrap().cast()) };

			cache_stat[index] = CacheStat::hand_made(count, total, 0);
			assert_eq!(cache.stat(), CacheAllocatorStat::hand_made(cache_stat));
		}
	}

	#[ktest(cache_allocator)]
	fn alloc_twice() {
		let multiplier = 8;
		let index = multiplier - MIN_CACHE_SIZE_MULTIPLIER;

		let mut cache = CacheAllocator::initialized();
		let mut cache_stat = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));

		let (ptr, total, count) = {
			let allocator = cache.get(index);

			let total = max(7, PAGE_SIZE / allocator.size() - 1);
			let count = max(allocator.size() * 7, PAGE_SIZE).next_power_of_two() / PAGE_SIZE;
			([allocator.allocate(), allocator.allocate()], total, count)
		};

		cache_stat[index] = CacheStat::hand_made(count, total, 2);
		assert_eq!(cache.stat(), CacheAllocatorStat::hand_made(cache_stat));

		// if not dealloc, then panic! will be called.
		unsafe { cache.get(index).deallocate(ptr[0].unwrap().cast()) };
		unsafe { cache.get(index).deallocate(ptr[1].unwrap().cast()) };

		cache_stat[index] = CacheStat::hand_made(count, total, 0);
		assert_eq!(cache.stat(), CacheAllocatorStat::hand_made(cache_stat));
	}
}
