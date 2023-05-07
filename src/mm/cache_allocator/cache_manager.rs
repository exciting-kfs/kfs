use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use super::{
	size_cache::SizeCache,
	traits::{CacheInit, CacheTrait},
	util::no_alloc_list::{NAList, Node},
};

type Result<T> = core::result::Result<T, AllocError>;

pub static mut CM: CacheManager<'static> = CacheManager::new();

const CACHE_ALLOCATOR_SIZE: usize = size_of::<SizeCache<42>>();
const NODE_SIZE: usize = size_of::<Node<NonNull<dyn CacheTrait>>>();

pub struct CacheManager<'a> {
	cache_space: SizeCache<'a, CACHE_ALLOCATOR_SIZE>,
	node_space: SizeCache<'a, NODE_SIZE>,
	list: NAList<NonNull<dyn CacheTrait>>,
}

impl<'a> CacheManager<'a> {
	pub const fn new() -> Self {
		CacheManager {
			cache_space: SizeCache::new(),
			node_space: SizeCache::new(),
			list: NAList::new(),
		}
	}

	pub fn new_allocator<A>(&mut self) -> Result<NonNull<A>>
	where
		A: CacheTrait + CacheInit + 'static, // TODO why static?
	{
		let mem_cache = self.cache_space.alloc()?;
		let cache = unsafe { A::construct_at(mem_cache) };

		match self.register(cache) {
			Ok(_) => Ok(mem_cache.cast()),
			Err(e) => unsafe {
				self.cache_space.dealloc(mem_cache);
				Err(e)
			},
		}
	}

	pub fn register(&mut self, cache: &'static mut dyn CacheTrait) -> Result<()> {
		// TODO why static?
		let mem_node = self.node_space.alloc()?.as_ptr();
		let node = unsafe { init_list_node(mem_node, cache as *mut dyn CacheTrait) };

		self.list.push_front(node);
		Ok(())
	}

	/// Safety
	///
	/// `ptr` must point cache alloctor.
	pub unsafe fn drop_allocator<A>(&mut self, ptr: NonNull<A>)
	where
		A: CacheTrait + 'static,
	{
		let cache = &mut *(ptr.as_ptr() as *mut dyn CacheTrait);
		cache.cache_shrink();
		if !cache.empty() {
			panic!("It can cause memory leak!");
		}

		self.list.find(|n| n.as_ref() == cache).map(|cache_ptr| {
			self.cache_space.dealloc(cache_ptr.cast());
		});

		self.unregister(cache);
	}

	pub fn unregister(&mut self, cache: &'static mut dyn CacheTrait) {
		unsafe {
			let node_ptr = self.list.remove_if(|n| n.as_ref() == cache);
			node_ptr.map(|node_ptr| {
				self.node_space.dealloc(node_ptr.cast());
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

#[macro_export]
macro_rules! kmem_cache_register {
	($cache:ident) => {
		let mut err_count = 0;
		for _ in 0..$crate::mm::cache_allocator::REGISTER_TRY {
			match $crate::mm::cache_allocator::CM.register(&mut $cache) {
				Ok(_) => break,
				Err(_) => {
					// pr_debug;
					err_count += 1;
					$crate::mm::cache_allocator::CM.cache_shrink();
				}
			}
		}
		if err_count == $crate::mm::cache_allocator::REGISTER_TRY {
			$crate::pr_info!("cache_manager: register: out of memory.");
			panic!(); // TODO 이게 맞나..?
		}
	};
}

mod tests {
	use kfs_macro::ktest;

	use super::CacheManager;
	use crate::mm::cache_allocator::size_cache::{tests::head_check, SizeCache};

	#[ktest]
	fn test_cache_alloc_dealloc() {
		let mut cm = CacheManager::new();
		let ptr = cm.new_allocator::<SizeCache<2048>>().unwrap();
		let _ = unsafe { &mut *(ptr.as_ptr()) };

		head_check(&mut cm.cache_space, 1, 0);
		head_check(&mut cm.node_space, 1, 0);
		assert_eq!(1, cm.list.count());

		unsafe { cm.drop_allocator(ptr) };

		head_check(&mut cm.node_space, 0, 0);
		head_check(&mut cm.cache_space, 0, 0);
		assert_eq!(0, cm.list.count());
	}

	static mut SIZE_CACHE: SizeCache<1024> = SizeCache::new();

	#[ktest]
	fn test_register_unregister() {
		let mut cm = CacheManager::new();

		cm.register(unsafe { &mut SIZE_CACHE }).unwrap();

		head_check(&mut cm.node_space, 1, 0);
		assert_eq!(1, cm.list.count());

		cm.unregister(unsafe { &mut SIZE_CACHE });

		head_check(&mut cm.node_space, 0, 0);
		assert_eq!(0, cm.list.count());
	}
}
