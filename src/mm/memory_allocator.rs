pub mod mem_atomic;
pub mod mem_normal;
pub mod util;

use core::alloc::{AllocError, Allocator, Layout};
use core::cell::UnsafeCell;
use core::ptr::NonNull;

use self::util::{level_of, LEVEL_END};

use super::cache_allocator::{alloc_block_from_page_alloc, dealloc_block_to_page_alloc};
use super::cache_allocator::{CacheAllocator, CacheAllocatorStat};
use super::page_allocator::MAX_RANK;
use super::GFP;

#[derive(Debug)]
pub struct MemoryAllocator {
	cache: UnsafeCell<CacheAllocator>,
	rank_count: UnsafeCell<[usize; MAX_RANK + 1]>,
}

impl MemoryAllocator {
	pub const fn new() -> Self {
		let rank_count = UnsafeCell::new([0; MAX_RANK + 1]);

		MemoryAllocator {
			cache: UnsafeCell::new(CacheAllocator::new()),
			rank_count,
		}
	}

	pub fn statistic(&self) -> MemoryAllocatorStat {
		let (cache, rank) = unsafe { self.get_fields() };

		let rank_stat = rank.clone();
		let cache_stat = cache.statistic();

		MemoryAllocatorStat {
			cache_stat,
			rank_stat,
		}
	}

	unsafe fn get_fields(&self) -> (&mut CacheAllocator, &mut [usize; MAX_RANK + 1]) {
		(
			self.cache.get().as_mut().unwrap(),
			self.rank_count.get().as_mut().unwrap(),
		)
	}
}

impl Drop for MemoryAllocator {
	fn drop(&mut self) {
		if (self.rank_count.get_mut()).iter().sum::<usize>() != 0 {
			panic!("It can cause memory leak!");
		}
	}
}

unsafe impl Allocator for MemoryAllocator {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let (cache, rank_count) = unsafe { self.get_fields() };

		let level = level_of(layout);
		match level.checked_sub(LEVEL_END) {
			None => cache.allocate(level),
			Some(rank) => {
				rank_count[rank] += 1;
				alloc_block_from_page_alloc(rank, GFP::Normal)
			}
		}
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let (cache, rank_count) = self.get_fields();

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
		let normal = MemoryAllocator::new();
		let cache = core::array::from_fn(|_| CacheStat::hand_made(0, 0, 0));
		let ca_stat = CacheAllocatorStat::hand_made(false, cache);
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
			let ca_stat = CacheAllocatorStat::hand_made(false, cache);
			let mut rank_count = [0; MAX_RANK + 1];
			let normal = MemoryAllocator::new();

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
		let ca_stat = CacheAllocatorStat::hand_made(false, cache);
		let mut rank_count = [0; MAX_RANK + 1];
		let normal = MemoryAllocator::new();

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

	use alloc::vec::Vec;

	#[ktest]
	fn with_collection() {
		let normal = MemoryAllocator::new();
		{
			let mut v = Vec::new_in(normal);
			for _ in 0..1000000 {
				v.push(1);
			}
		}
	}
}
