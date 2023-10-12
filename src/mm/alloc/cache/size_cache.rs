use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::mm::alloc::{page, Zone};
use crate::mm::page::ptr_to_allocated_page;
use crate::mm::{constant::*, util::*};
use crate::sync::Locked;
use crate::trace_feature;

use super::meta_cache::MetaCache;
use super::no_alloc_list::{NAList, Node};
use super::traits::{CacheInit, CacheStat, CacheTrait};

type Result<T> = core::result::Result<T, AllocError>;

#[derive(Debug)]
pub struct SizeCache<const N: usize> {
	partial: NAList<MetaCache>,
	page_count: usize,
}

impl<const N: usize> SizeCache<N> {
	const DEFAULT_COUNT: usize = 7;
	const SIZE: usize = align_with_hw_cache(N);
	const RANK: usize = rank_of(Self::SIZE, Self::DEFAULT_COUNT);

	pub const fn new() -> Self {
		SizeCache {
			partial: NAList::new(),
			page_count: 0,
		}
	}

	pub fn reserve(&mut self, count: usize) -> Result<()> {
		let rank = rank_of(Self::SIZE, count);
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
			.first()
			.filter(|e| unsafe { !(*e).as_ref().is_full() })
			.or_else(|| unsafe {
				let page = self.alloc_pages(Self::RANK).ok()?;
				let node = Node::alloc_at(page.cast());
				self.partial.push_front(node);

				let meta_cache = MetaCache::construct_at(page.cast(), Self::SIZE);
				Some(NonNull::new_unchecked(meta_cache))
			})
	}

	fn alloc_pages(&mut self, rank: usize) -> Result<NonNull<[u8]>> {
		let page = page::alloc_pages(rank, Zone::Normal)?;
		self.page_count += 1 << rank;
		Ok(page)
	}

	fn dealloc_pages(&mut self, meta: &mut MetaCache) {
		unsafe {
			self.page_count -= 1 << meta.rank();
			let ptr = meta as *mut MetaCache;
			let ptr = NonNull::new_unchecked(ptr.cast());
			page::free_pages(ptr);
		}
	}
}

impl<const N: usize> CacheInit for Locked<SizeCache<N>> {}

impl<const N: usize> CacheTrait for Locked<SizeCache<N>> {
	#[inline]
	fn empty(&self) -> bool {
		self.lock().partial.first() == None
	}

	#[inline]
	fn size(&self) -> usize {
		SizeCache::<N>::SIZE
	}

	fn stat(&self) -> CacheStat {
		let (total, inuse) = self
			.lock()
			.partial
			.iter()
			.fold((0, 0), |(mut t, mut i), m| {
				i += m.inuse();
				t += m.total();
				(t, i)
			});

		CacheStat {
			page_count: self.lock().page_count,
			total,
			inuse,
		}
	}

	fn contains(&mut self, ptr: NonNull<u8>) -> bool {
		self.lock()
			.partial
			.find(|meta_cache| meta_cache.contains(ptr))
			.is_some()
	}

	fn allocate(&mut self) -> Result<NonNull<[u8]>> {
		let mut size_cache = self.lock();
		let mut meta_cache_ptr = size_cache.get_meta_cache().ok_or(AllocError)?;

		let meta_cache = unsafe { meta_cache_ptr.as_mut() };
		let ret = meta_cache.alloc();

		if size_cache
			.partial
			.first()
			.is_some_and(|h| unsafe { h.as_ref() }.is_full())
		{
			size_cache.partial.head_to_next();
		}

		ret
	}

	/// # Safety
	///
	/// `ptr` must point a memory block allocated by `self`.
	unsafe fn deallocate(&mut self, ptr: NonNull<u8>) {
		let mut size_cache = self.lock();

		let page = match ptr_to_allocated_page(ptr) {
			Some(p) => p,
			None => return,
		};

		let mut node = page.cast::<Node<MetaCache>>();

		size_cache.partial.remove(node.as_mut());

		let meta_cache = node.as_mut().data_mut();
		meta_cache.dealloc(ptr);

		if meta_cache.is_free() && size_cache.page_count >= MAX_CACHE_PAGE_PER_ALLOCATOR {
			size_cache.dealloc_pages(meta_cache);
		} else {
			size_cache.partial.push_front(node.as_mut());
		}
	}

	fn cache_shrink(&mut self) {
		let mut size_cache = self.lock();

		trace_feature!(
			"oom",
			"size_cache<{}>: before shrink: page_count: {}",
			SizeCache::<N>::SIZE,
			size_cache.page_count
		);

		let m_cache_list = &mut size_cache.partial;

		let (mut satisfied, not) = m_cache_list.iter_mut().partition(|m| m.is_free());
		(*m_cache_list) = not;

		satisfied
			.iter_mut()
			.for_each(|meta_cache| size_cache.dealloc_pages(meta_cache));

		trace_feature!(
			"oom",
			"size_cache<{}>: after shrink: page_count: {}",
			SizeCache::<N>::SIZE,
			size_cache.page_count
		);
	}
}

impl<const N: usize> Default for Locked<SizeCache<N>> {
	fn default() -> Self {
		trace_feature!("size_cache", "size_cache<{}> generated", N);
		Locked::new(SizeCache::<N>::new())
	}
}

const fn rank_of(size: usize, count: usize) -> usize {
	const NODE_SIZE: usize = size_of::<Node<MetaCache>>();
	const META_SIZE: usize = align_with_hw_cache(NODE_SIZE);

	let size = size * count + META_SIZE;
	let page_align = next_align(size, PAGE_SIZE);
	let page_size = page_align.next_power_of_two();

	size_to_rank(page_size)
}

pub mod tests {
	use super::*;
	use core::ptr::NonNull;
	use kfs_macro::ktest;

	pub fn get_head<const N: usize>(cache: &mut SizeCache<N>) -> &mut MetaCache {
		let ret = unsafe { cache.partial.first().unwrap().as_mut() };
		ret
	}

	pub fn head_check<const N: usize>(cache: &mut SizeCache<N>, inuse: usize, rank: usize) {
		let head = get_head(cache);
		let max = (size_of_rank(head.rank()) - MetaCache::META_SIZE) / head.cache_size;

		assert_eq!(head.free_list.count(), max - inuse);
		assert_eq!(head.inuse(), inuse);
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
		let mut cache = Locked::new(SizeCache::<SIZE>::new());

		for i in 0..MAX_COUNT {
			// 64 * 63 => 4032 + meta_cache(64) => full
			let _ = cache.allocate();
			assert_eq!(cache.lock().page_count, 1);
			assert_eq!(cache.lock().partial.count(), 1);

			head_check(&mut cache.lock(), i + 1, 0);
		}

		let _ = cache.allocate();
		assert_eq!(cache.lock().page_count, 2);
		assert_eq!(cache.lock().partial.count(), 2);

		head_check(&mut cache.lock(), 1, 0);
	}

	#[ktest]
	fn test_dealloc() {
		const SIZE: usize = 60;
		const END: usize = 63;
		let mut cache = Locked::new(SizeCache::<SIZE>::new());

		// dealloc one when the inuse is 1.
		let ptr = cache.allocate();
		head_check(&mut cache.lock(), 1, 0);
		unsafe { cache.deallocate(ptr.unwrap().cast()) };

		assert_eq!(cache.lock().partial.count(), 1);
		head_check(&mut cache.lock(), 0, 0);

		// dealloc one when the inuse of 2nd memory block is 1.
		let mut ptrs: [NonNull<u8>; 64] = [NonNull::dangling(); 64];
		for i in 0..END {
			ptrs[i] = cache.allocate().unwrap().cast();
		}

		let ptr = cache.allocate();
		unsafe { cache.deallocate(ptr.unwrap().cast()) };

		assert_eq!(cache.lock().partial.count(), 2);
		head_check(&mut cache.lock(), 0, 0);

		// dealloc whole in one memory block.
		for i in 0..END {
			let ptr = ptrs[i];
			unsafe { cache.deallocate(ptr.cast()) };

			let head = unsafe { cache.lock().partial.first().unwrap().as_mut() };
			assert_eq!(head.free_list.count(), i + 1);
			assert_eq!(head.inuse(), END - (i + 1));
		}
	}

	#[ktest]
	fn test_alloc_bound() {
		fn do_test<const N: usize>(rank: usize) {
			let mut cache = Locked::new(SizeCache::<N>::new());
			let _ = cache.allocate();
			head_check(&mut cache.lock(), 1, rank);
		}
		do_test::<1024>(1);
		do_test::<2048>(2);
		do_test::<4096>(3);
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

		fn meta_cache_count_check(cache: &mut Locked<SizeCache<SIZE>>, previous: usize) {
			assert_eq!(cache.lock().partial.count(), previous);
			cache.cache_shrink();
			assert_eq!(cache.lock().partial.count(), 0);
		}

		// shrink one memory block.
		let mut cache = Locked::new(SizeCache::<SIZE>::new());
		let ptr = cache.allocate().unwrap();
		unsafe { cache.deallocate(ptr.cast()) }
		meta_cache_count_check(&mut cache, 1);

		// shrink two memory block.
		const MAX_COUNT: usize = SizeCache::<SIZE>::DEFAULT_COUNT + 1;
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
