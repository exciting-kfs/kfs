pub mod mem_atomic;
pub mod mem_normal;
pub mod util;

use core::alloc::{AllocError, Layout};
use core::ptr::NonNull;

use self::util::{level_of, LEVEL_END};

use super::cache_allocator::{alloc_block_from_page_alloc, dealloc_block_to_page_alloc};
use super::cache_allocator::{CacheAllocator, CacheAllocatorStat};
use super::page_allocator::MAX_RANK;
use super::GFP;

#[derive(Debug)]
pub struct MemoryAllocator {
	cache: CacheAllocator,
	rank_count: [usize; MAX_RANK + 1],
}

impl MemoryAllocator {
	pub const fn uninit() -> Self {
		let rank_count = [0; MAX_RANK + 1];

		MemoryAllocator {
			cache: CacheAllocator::uninit(),
			rank_count,
		}
	}

	pub fn init(&mut self) {
		self.cache.init();
	}

	pub fn initialized() -> Self {
		let mut allocator = MemoryAllocator::uninit();
		allocator.init();
		allocator
	}

	pub fn statistic(&mut self) -> MemoryAllocatorStat {
		let (cache, rank) = (&mut self.cache, &mut self.rank_count);

		let rank_stat = rank.clone();
		let cache_stat = cache.statistic();

		MemoryAllocatorStat {
			cache_stat,
			rank_stat,
		}
	}

	fn allocate(&mut self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let (cache, rank_count) = (&mut self.cache, &mut self.rank_count);

		let level = level_of(layout);
		match level.checked_sub(LEVEL_END) {
			None => cache.allocate(level),
			Some(rank) => {
				rank_count[rank] += 1;
				alloc_block_from_page_alloc(rank, GFP::Normal)
			}
		}
	}

	unsafe fn deallocate(&mut self, ptr: NonNull<u8>, layout: Layout) {
		let (cache, rank_count) = (&mut self.cache, &mut self.rank_count);

		let level = level_of(layout);
		match level.checked_sub(LEVEL_END) {
			None => cache.deallocate(ptr, level),
			Some(rank) => {
				rank_count[rank] -= 1;
				dealloc_block_to_page_alloc(ptr);
			}
		}
	}
}

impl Drop for MemoryAllocator {
	fn drop(&mut self) {
		if self.rank_count.iter().sum::<usize>() != 0 {
			panic!("It can cause memory leak!");
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryAllocatorStat {
	cache_stat: CacheAllocatorStat,
	rank_stat: [usize; MAX_RANK + 1],
}

impl MemoryAllocatorStat {
	pub const fn hand_made(
		cache_stat: CacheAllocatorStat,
		rank_count: [usize; MAX_RANK + 1],
	) -> Self {
		Self {
			cache_stat,
			rank_stat: rank_count,
		}
	}
}

mod tests {
	use crate::mm::{cache_allocator::CacheStat, constant::PAGE_SHIFT};

	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	fn new() {
		let mut normal = MemoryAllocator::initialized();
		let cache = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
		let ca_stat = CacheAllocatorStat::hand_made(cache);
		assert_eq!(
			normal.statistic(),
			MemoryAllocatorStat::hand_made(ca_stat, [0; MAX_RANK + 1])
		)
	}

	#[ktest]
	fn alloc_dealloc() {
		for rank in 0..=MAX_RANK {
			let layout =
				unsafe { Layout::from_size_align_unchecked(1 << (rank + PAGE_SHIFT), 4096) };
			let cache = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
			let ca_stat = CacheAllocatorStat::hand_made(cache);
			let mut rank_count = [0; MAX_RANK + 1];
			let mut normal = MemoryAllocator::initialized();

			let ptr = normal.allocate(layout);

			rank_count[rank] = 1;
			assert_eq!(
				normal.statistic(),
				MemoryAllocatorStat::hand_made(ca_stat, rank_count)
			);

			// if not dealloc, then panic! will be called.
			unsafe { normal.deallocate(ptr.unwrap().cast(), layout) };

			rank_count[rank] = 0;
			assert_eq!(
				normal.statistic(),
				MemoryAllocatorStat::hand_made(ca_stat, rank_count)
			);
		}
	}

	#[ktest]
	fn alloc_twice() {
		let rank = 2;
		let layout = unsafe { Layout::from_size_align_unchecked(1 << (rank + PAGE_SHIFT), 4096) };
		let cache = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
		let ca_stat = CacheAllocatorStat::hand_made(cache);
		let mut rank_count = [0; MAX_RANK + 1];
		let mut normal = MemoryAllocator::initialized();

		let ptr = [normal.allocate(layout), normal.allocate(layout)];

		rank_count[rank] = 2;
		assert_eq!(
			normal.statistic(),
			MemoryAllocatorStat::hand_made(ca_stat, rank_count)
		);

		// if not dealloc, then panic! will be called.
		unsafe { normal.deallocate(ptr[0].unwrap().cast(), layout) };
		unsafe { normal.deallocate(ptr[1].unwrap().cast(), layout) };

		rank_count[rank] = 0;
		assert_eq!(
			normal.statistic(),
			MemoryAllocatorStat::hand_made(ca_stat, rank_count)
		);
	}
}
