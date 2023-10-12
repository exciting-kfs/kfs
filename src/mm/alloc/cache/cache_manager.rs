use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use crate::sync::Locked;

use super::{
	no_alloc_list::{NAList, Node},
	size_cache::SizeCache,
	traits::{CacheInit, CacheTrait},
};

type Result<T> = core::result::Result<T, AllocError>;

pub static mut CM: CacheManager = CacheManager::new();

const CACHE_ALLOCATOR_SIZE: usize = size_of::<Locked<SizeCache<42>>>();
const NODE_SIZE: usize = size_of::<Node<NonNull<dyn CacheTrait>>>();

pub struct CacheManager {
	cache_space: Locked<SizeCache<CACHE_ALLOCATOR_SIZE>>,
	node_space: Locked<SizeCache<NODE_SIZE>>,
	list: NAList<NonNull<dyn CacheTrait>>,
}

impl CacheManager {
	pub const fn new() -> Self {
		CacheManager {
			cache_space: Locked::new(SizeCache::new()),
			node_space: Locked::new(SizeCache::new()),
			list: NAList::new(),
		}
	}

	pub fn new_allocator<A>(&mut self) -> Result<NonNull<A>>
	where
		A: CacheTrait + CacheInit + 'static,
	{
		let mem_cache = self.cache_space.allocate()?;
		let cache = unsafe { A::construct_at(mem_cache.cast()) };

		match self.register(cache) {
			Ok(_) => Ok(mem_cache.cast()),
			Err(e) => unsafe {
				self.cache_space.deallocate(mem_cache.cast());
				Err(e)
			},
		}
	}

	pub fn register(&mut self, cache: &'static mut dyn CacheTrait) -> Result<()> {
		let mem_node = self.node_space.allocate()?.as_ptr();
		let cache = cache as *mut dyn CacheTrait;
		let node = unsafe { init_list_node(mem_node.cast(), cache) };

		self.list.push_front(node);
		Ok(())
	}

	/// # Safety
	///
	/// `ptr` must point cache alloctor.
	pub unsafe fn drop_allocator(&mut self, ptr: NonNull<dyn CacheTrait>) {
		let cache = &mut *(ptr.as_ptr() as *mut dyn CacheTrait);
		cache.cache_shrink();

		if !cache.empty() {
			panic!("It can cause memory leak!");
		}

		self.list.find(|n| n.as_ref() == cache).map(|cache_ptr| {
			self.cache_space.deallocate(cache_ptr.cast());
		});

		self.unregister(cache);
	}

	pub fn unregister(&mut self, cache: &'static mut dyn CacheTrait) {
		unsafe {
			let node_ptr = self.list.remove_if(|n| n.as_ref() == cache);
			node_ptr.map(|node_ptr| {
				self.node_space.deallocate(node_ptr.cast());
			});
		}
	}

	pub fn cache_shrink(&mut self) {
		self.cache_space.cache_shrink();
		self.node_space.cache_shrink();
		self.list.iter_mut().for_each(|ptr| {
			let cache = unsafe { ptr.as_mut() };
			cache.cache_shrink();
		})
	}

	pub fn cache_size(&mut self, ptr: NonNull<u8>) -> Option<usize> {
		for ca in self.list.iter_mut() {
			let allocator = unsafe { ca.as_mut() };
			if allocator.contains(ptr) {
				return Some(allocator.size());
			}
		}
		None
	}
}

/// # Safety
///
/// * `cache` pointer must not be null.
/// * The memory pointed by `mem_node` must be reserved for Node initialization.
unsafe fn init_list_node<'a>(
	mem_node: *mut u8,
	cache: *mut dyn CacheTrait,
) -> &'a mut Node<NonNull<dyn CacheTrait>> {
	let data = NonNull::new_unchecked(cache);
	let ptr = NonNull::new_unchecked(mem_node);
	Node::construct_at(ptr, data)
}

mod tests {
	use super::*;
	use crate::mm::alloc::cache::size_cache::tests::head_check;
	use kfs_macro::ktest;

	#[ktest]
	fn test_cache_alloc_dealloc() {
		let mut cm = CacheManager::new();
		let ptr = cm.new_allocator::<Locked<SizeCache<2048>>>().unwrap();
		let _ = unsafe { &mut *(ptr.as_ptr()) };

		head_check(&mut cm.cache_space.lock(), 1, 0);
		head_check(&mut cm.node_space.lock(), 1, 0);
		assert_eq!(1, cm.list.count());

		unsafe { cm.drop_allocator(ptr) };

		head_check(&mut cm.node_space.lock(), 0, 0);
		head_check(&mut cm.cache_space.lock(), 0, 0);
		assert_eq!(0, cm.list.count());
	}

	static mut SIZE_CACHE: Locked<SizeCache<1024>> = Locked::new(SizeCache::new());

	#[ktest]
	fn test_register_unregister() {
		let mut cm = CacheManager::new();

		cm.register(unsafe { &mut SIZE_CACHE }).unwrap();

		head_check(&mut cm.node_space.lock(), 1, 0);
		assert_eq!(1, cm.list.count());

		cm.unregister(unsafe { &mut SIZE_CACHE });

		head_check(&mut cm.node_space.lock(), 0, 0);
		assert_eq!(0, cm.list.count());
	}
}

pub fn oom_handler() {
	unsafe { CM.cache_shrink() };
}
