pub mod meta_cache;

use core::alloc::AllocError;
use core::marker::PhantomData;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::mm::slub::no_alloc_list::Node;

use super::cache::{align_with_hw_cache, CacheBase, CacheInit};
use super::no_alloc_list::NAList;
use super::{alloc_block_from_page_alloc, PAGE_SIZE};

use self::meta_cache::MetaCache;

type Result<T> = core::result::Result<T, AllocError>;

#[derive(Debug)]
pub struct SizeCache<'page, const N: usize> {
	partial: NAList<MetaCache>,
	page_count: usize,
	phantom: PhantomData<&'page usize>,
}

impl<'page, const N: usize> SizeCache<'page, N> {
	const SIZE: usize = align_with_hw_cache(N);
	const RANK: usize = rank_of(Self::SIZE);

	pub const fn new() -> Self {
		SizeCache {
			partial: NAList::new(),
			page_count: 0,
			phantom: PhantomData,
		}
	}

	pub fn reserve(&mut self, count: usize) -> Result<()> {
		let rank = rank_of(Self::SIZE * count);
		let page = self.alloc_pages(rank)?;
		unsafe {
			let node = Node::alloc_at(page.0);
			self.partial.push_front(node);
			MetaCache::construct_at(page.0, Self::SIZE);
		};
		Ok(())
	}

	pub fn alloc(&mut self) -> Result<NonNull<u8>> {
		let meta_cache = self.partial.head().and_then(|mut meta_cache_ptr| {
			let meta_cache = unsafe { meta_cache_ptr.as_mut() };
			match meta_cache.is_full() {
				true => None,
				false => Some(meta_cache_ptr),
			}
		});

		meta_cache
			.or_else(|| {
				let page = self.alloc_pages(Self::RANK).ok()?;
				let ptr = unsafe {
					let node = Node::alloc_at(page.0);
					self.partial.push_front(node);

					let meta_cache = MetaCache::construct_at(page.0, Self::SIZE);
					NonNull::new_unchecked(meta_cache)
				};
				Some(ptr)
			})
			.map(|mut meta_cache_ptr| {
				let meta_cache = unsafe { meta_cache_ptr.as_mut() };
				meta_cache.alloc().unwrap()
			})
			.ok_or(AllocError)
	}

	/// Safety
	///
	/// `ptr` must point a memory block allocated by `self`.
	pub unsafe fn dealloc(&mut self, ptr: NonNull<u8>) {
		self.partial
			.find(|meta_cache| meta_cache.contains(ptr))
			.map(|meta_cache| {
				meta_cache.dealloc(ptr);
			});
	}

	fn alloc_pages(&mut self, rank: usize) -> Result<(NonNull<u8>, usize)> {
		let page = alloc_block_from_page_alloc(rank)?;
		self.page_count += 1 << rank;
		Ok(page)
	}
}

impl<'page, const N: usize> CacheBase for SizeCache<'_, N> {
	fn partial(&mut self) -> &mut NAList<MetaCache> {
		&mut self.partial
	}

	fn empty(&self) -> bool {
		self.partial.head() == None
	}
}

impl<'page, const N: usize> Default for SizeCache<'_, N> {
	fn default() -> Self {
		Self::new()
	}
}

impl<'page, const N: usize> CacheInit for SizeCache<'_, N> {}

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

pub trait SizeCacheTrait {
	fn allocate(&mut self) -> *mut u8;
	unsafe fn deallocate(&mut self, ptr: *mut u8);
}

impl<'page, const N: usize> SizeCacheTrait for SizeCache<'page, N> {
	fn allocate(&mut self) -> *mut u8 {
		match self.alloc() {
			Ok(ptr) => ptr.as_ptr(),
			Err(_) => 0 as *mut u8,
		}
	}

	unsafe fn deallocate(&mut self, ptr: *mut u8) {
		self.dealloc(NonNull::new_unchecked(ptr));
	}
}

mod tests {
	use core::ptr::NonNull;

	use kfs_macro::ktest;

	use crate::mm::slub::cache::CacheBase;

	use super::SizeCache;

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
			let _ = cache.alloc();
			let head = unsafe { cache.partial.head().unwrap().as_mut() };
			assert_eq!(cache.page_count, 1);
			assert_eq!(cache.partial.count(), 1);
			assert_eq!(head.free_list.count(), MAX_COUNT - (i + 1));
			assert_eq!(head.inuse, i + 1);
		}

		let _ = cache.alloc();
		let head = unsafe { cache.partial.head().unwrap().as_mut() };
		assert_eq!(cache.page_count, 2);
		assert_eq!(cache.partial.count(), 2);
		assert_eq!(head.free_list.count(), 62);
		assert_eq!(head.inuse, 1);
	}

	#[ktest]
	fn test_dealloc() {
		const SIZE: usize = 60;
		const MAX_COUNT: usize = 63;
		let mut cache = SizeCache::<SIZE>::new();

		// dealloc one when the inuse is 1.
		let ptr = cache.alloc();
		unsafe { cache.dealloc(ptr.unwrap()) };

		let head = unsafe { cache.partial.head().unwrap().as_mut() };
		assert_eq!(head.free_list.count(), MAX_COUNT);
		assert_eq!(head.inuse, 0);

		// dealloc one when the inuse of 2nd memory block is 1.
		let mut ptrs: [NonNull<u8>; 64] = [NonNull::dangling(); 64];
		for i in 0..MAX_COUNT {
			ptrs[i] = cache.alloc().unwrap();
		}

		let ptr = cache.alloc();
		unsafe { cache.dealloc(ptr.unwrap()) };

		let head = unsafe { cache.partial.head().unwrap().as_mut() };
		assert_eq!(head.free_list.count(), MAX_COUNT);
		assert_eq!(head.inuse, 0);
		assert_eq!(cache.partial.count(), 2);

		// dealloc whole in one memory block.
		for i in 0..MAX_COUNT {
			let ptr = ptrs[i];
			unsafe { cache.dealloc(ptr) };

			let last = cache.partial.iter().last().unwrap();
			assert_eq!(last.free_list.count(), i + 1);
			assert_eq!(last.inuse, MAX_COUNT - (i + 1));
		}
	}

	#[ktest]
	fn test_alloc_bound() {
		// size = 4080
	}

	#[ktest]
	fn test_reserve() {
		const SIZE: usize = 2000;
		let mut cache = SizeCache::<SIZE>::new();

		// reserve for one cache.
		// META_CACHE_SIZE + SizeCache<SIZE>::SIZE = 32 + 2048 = 2032 => rank 0
		cache.reserve(1).unwrap();
		assert_eq!(cache.partial.count(), 1);
		let head = unsafe { cache.partial.head().unwrap().as_mut() };
		assert_eq!(head.rank(), 0);
		assert_eq!(head.free_list.count(), 1);

		//reserve for three cache.
		// META_CACHE_SIZE + SizeCache<SIZE>::SIZE * 3 = 32 + 2048 * 3 = 6176 => rank 1
		cache.reserve(3).unwrap();
		assert_eq!(cache.partial.count(), 2);
		let head = unsafe { cache.partial.head().unwrap().as_mut() };
		assert_eq!(head.rank(), 1);
		assert_eq!(head.free_list.count(), 3);
	}

	#[ktest]
	fn test_shrink() {
		// shrink one memory block.
		let mut cache = SizeCache::<1024>::new();
		let ptr = cache.alloc().unwrap();
		unsafe { cache.dealloc(ptr) }
		assert_eq!(cache.partial.count(), 1);
		cache.cache_shrink();
		assert_eq!(cache.partial.count(), 0);

		// shrink two memory block.
		const MAX_COUNT: usize = 3;
		let mut ptrs: [NonNull<u8>; 3] = [NonNull::dangling(); 3];
		for i in 0..MAX_COUNT {
			ptrs[i] = cache.alloc().unwrap();
		}
		let ptr = cache.alloc().unwrap();
		unsafe {
			cache.dealloc(ptr);

			for i in 0..MAX_COUNT {
				cache.dealloc(ptrs[i]);
			}
		}

		assert_eq!(cache.partial.count(), 2);
		cache.cache_shrink();
		assert_eq!(cache.partial.count(), 0);
	}
}
