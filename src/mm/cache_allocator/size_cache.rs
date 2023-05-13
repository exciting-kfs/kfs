use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::mm::constant::PAGE_SIZE;
use crate::mm::GFP;

use super::traits::{CacheInit, CacheStat, CacheTrait};
use super::util::no_alloc_list::{NAList, Node};
use super::util::{align_with_hw_cache, alloc_block_from_page_alloc};

use super::meta_cache::MetaCache;

type Result<T> = core::result::Result<T, AllocError>;

#[derive(Debug)]
pub struct SizeCache<const N: usize> {
	partial: NAList<MetaCache>,
	page_count: usize,
}

impl<const N: usize> SizeCache<N> {
	const SIZE: usize = align_with_hw_cache(N);
	const RANK: usize = rank_of(Self::SIZE);

	pub const fn new() -> Self {
		SizeCache {
			partial: NAList::new(),
			page_count: 0,
		}
	}

	pub fn reserve(&mut self, count: usize) -> Result<()> {
		let rank = rank_of(Self::SIZE * count);
		let page = self.alloc_pages(rank)?;
		unsafe {
			let node = Node::alloc_at(page.cast());
			self.partial.push_front(node);
			MetaCache::construct_at(page.cast(), Self::SIZE);
		};
		Ok(())
	}

	fn get_meta_cache(&mut self) -> Option<NonNull<MetaCache>> {
		self.partial
			.head()
			.and_then(|mut meta_cache_ptr| {
				let meta_cache = unsafe { meta_cache_ptr.as_mut() };
				match meta_cache.is_full() {
					true => None,
					false => Some(meta_cache_ptr),
				}
			})
			.or_else(|| unsafe {
				let page = self.alloc_pages(Self::RANK).ok()?;
				let node = Node::alloc_at(page.cast());
				self.partial.push_front(node);

				let meta_cache = MetaCache::construct_at(page.cast(), Self::SIZE);
				Some(NonNull::new_unchecked(meta_cache))
			})
	}

	fn alloc_pages(&mut self, rank: usize) -> Result<NonNull<[u8]>> {
		let page = alloc_block_from_page_alloc(rank, GFP::Normal)?;
		self.page_count += 1 << rank;
		Ok(page)
	}
}

impl<const N: usize> CacheInit for SizeCache<N> {}

impl<const N: usize> CacheTrait for SizeCache<N> {
	fn partial(&mut self) -> &mut NAList<MetaCache> {
		&mut self.partial
	}

	fn empty(&self) -> bool {
		self.partial.head() == None
	}

	fn statistic(&self) -> CacheStat {
		let (total, inuse) = self.partial.iter().fold((0, 0), |(mut t, mut i), m| {
			i += m.inuse;
			t += m.total();
			(t, i)
		});

		CacheStat {
			page_count: self.page_count,
			total,
			inuse,
		}
	}

	fn allocate(&mut self) -> Result<NonNull<[u8]>> {
		let mut meta_cache_ptr = self.get_meta_cache().ok_or(AllocError)?;
		let meta_cache = unsafe { meta_cache_ptr.as_mut() };
		meta_cache.alloc()
	}

	/// # Safety
	///
	/// `ptr` must point a memory block allocated by `self`.
	unsafe fn deallocate(&mut self, ptr: NonNull<u8>) {
		self.partial
			.remove_if(|meta_cache| meta_cache.contains(ptr))
			.map(|mut node| {
				let meta_cache = unsafe { node.cast::<MetaCache>().as_mut() };
				meta_cache.dealloc(ptr);

				let node = unsafe { node.as_mut() };
				self.partial.push_front(node);
			});
	}
}

impl<const N: usize> Default for SizeCache<N> {
	fn default() -> Self {
		Self::new()
	}
}

const fn rank_of(size: usize) -> usize {
	const NODE_SIZE: usize = size_of::<Node<MetaCache>>();
	const META_CACHE_SIZE: usize = align_with_hw_cache(NODE_SIZE);

	let size = size + META_CACHE_SIZE;
	let mut rank = 0;
	let mut count = (size - 1) / PAGE_SIZE;

	while count > 0 {
		count /= 2;
		rank += 1;
	}
	rank
}

pub mod tests {
	use super::*;
	use core::ptr::NonNull;
	use kfs_macro::ktest;

	use crate::mm::{
		cache_allocator::{meta_cache::MetaCache, traits::CacheTrait},
		util::size_of_rank,
	};

	pub fn get_head(cache: &mut dyn CacheTrait) -> &mut MetaCache {
		let ret = unsafe { cache.partial().head().unwrap().as_mut() };
		ret
	}

	pub fn head_check(cache: &mut dyn CacheTrait, inuse: usize, rank: usize) {
		let head = get_head(cache);
		let max = (size_of_rank(head.rank()) - MetaCache::META_SIZE) / head.cache_size;

		assert_eq!(head.free_list.count(), max - inuse);
		assert_eq!(head.inuse, inuse);
		assert_eq!(head.rank(), rank);
	}

	#[ktest]
	fn test_size() {
		const ACTUAL: [usize; 8] = [16, 16, 32, 32, 64, 64, 128, 192];

		assert_eq!(SizeCache::<0>::SIZE, ACTUAL[0]);
		assert_eq!(SizeCache::<16>::SIZE, ACTUAL[1]);
		assert_eq!(SizeCache::<17>::SIZE, ACTUAL[2]);
		assert_eq!(SizeCache::<32>::SIZE, ACTUAL[3]);
		assert_eq!(SizeCache::<33>::SIZE, ACTUAL[4]);
		assert_eq!(SizeCache::<64>::SIZE, ACTUAL[5]);
		assert_eq!(SizeCache::<65>::SIZE, ACTUAL[6]);
		assert_eq!(SizeCache::<129>::SIZE, ACTUAL[7]);
	}

	#[ktest]
	fn test_alloc() {
		const SIZE: usize = 60;
		const MAX_COUNT: usize = 63;
		let mut cache = SizeCache::<SIZE>::new();

		for i in 0..MAX_COUNT {
			// 64 * 63 => 4032 + meta_cache(16) => full
			let _ = cache.allocate();
			assert_eq!(cache.page_count, 1);
			assert_eq!(cache.partial.count(), 1);

			head_check(&mut cache, i + 1, 0);
		}

		let _ = cache.allocate();
		assert_eq!(cache.page_count, 2);
		assert_eq!(cache.partial.count(), 2);

		head_check(&mut cache, 1, 0);
	}

	#[ktest]
	fn test_dealloc() {
		const SIZE: usize = 60;
		const MAX_COUNT: usize = 63;
		let mut cache = SizeCache::<SIZE>::new();

		// dealloc one when the inuse is 1.
		let ptr = cache.allocate();
		head_check(&mut cache, 1, 0);
		unsafe { cache.deallocate(ptr.unwrap().cast()) };

		assert_eq!(cache.partial.count(), 1);
		head_check(&mut cache, 0, 0);

		// dealloc one when the inuse of 2nd memory block is 1.
		let mut ptrs: [NonNull<u8>; 64] = [NonNull::dangling(); 64];
		for i in 0..MAX_COUNT {
			ptrs[i] = cache.allocate().unwrap().cast();
		}

		let ptr = cache.allocate();
		unsafe { cache.deallocate(ptr.unwrap().cast()) };

		assert_eq!(cache.partial.count(), 2);
		head_check(&mut cache, 0, 0);

		// dealloc whole in one memory block.
		for i in 0..MAX_COUNT {
			let ptr = ptrs[i];
			unsafe { cache.deallocate(ptr.cast()) };

			let head = unsafe { cache.partial.head().unwrap().as_mut() };
			assert_eq!(head.free_list.count(), i + 1);
			assert_eq!(head.inuse, MAX_COUNT - (i + 1));
		}
	}

	#[ktest]
	fn test_alloc_bound() {
		fn do_test<const N: usize>(rank: usize) {
			let mut cache = SizeCache::<N>::new();
			let _ = cache.allocate();
			head_check(&mut cache, 1, rank);
		}
		do_test::<4032>(0);
		do_test::<4064>(1);
		do_test::<4080>(1);
	}

	#[ktest]
	fn test_reserve() {
		const SIZE: usize = 2000;
		let mut cache = SizeCache::<SIZE>::new();

		// reserve for one cache.
		// META_CACHE_SIZE + SizeCache<SIZE>::SIZE = 32 + 2048 = 2032 => rank 0
		cache.reserve(1).unwrap();
		assert_eq!(cache.partial.count(), 1);
		head_check(&mut cache, 0, 0);

		//reserve for three cache.
		// META_CACHE_SIZE + SizeCache<SIZE>::SIZE * 3 = 32 + 2048 * 3 = 6176 => rank 1
		cache.reserve(3).unwrap();
		assert_eq!(cache.partial.count(), 2);
		head_check(&mut cache, 0, 1);
	}

	#[ktest]
	fn test_shrink() {
		const SIZE: usize = 1024;

		fn meta_cache_count_check(cache: &mut SizeCache<SIZE>, previous: usize) {
			assert_eq!(cache.partial.count(), previous);
			cache.cache_shrink();
			assert_eq!(cache.partial.count(), 0);
		}

		// shrink one memory block.
		let mut cache = SizeCache::<SIZE>::new();
		let ptr = cache.allocate().unwrap();
		unsafe { cache.deallocate(ptr.cast()) }
		meta_cache_count_check(&mut cache, 1);

		// shrink two memory block.
		const MAX_COUNT: usize = 3;
		let mut ptrs: [NonNull<u8>; MAX_COUNT] = [NonNull::dangling(); MAX_COUNT];
		for i in 0..MAX_COUNT {
			ptrs[i] = cache.allocate().unwrap().cast();
		}
		let ptr = cache.allocate().unwrap();
		unsafe {
			cache.deallocate(ptr.cast());

			for i in 0..MAX_COUNT {
				cache.deallocate(ptrs[i]);
			}
		}
		meta_cache_count_check(&mut cache, 2);
	}
}
