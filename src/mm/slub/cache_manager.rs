use core::alloc::AllocError;
use core::mem::size_of;
use core::ptr::NonNull;

use super::{
	cache::{CacheBase, CacheInit},
	no_alloc_list::{NAList, Node},
	size_cache::SizeCache,
};

type Result<T> = core::result::Result<T, AllocError>;

pub static mut CM: CacheManager<'static> = CacheManager::new();

const CACHE_ALLOCATOR_SIZE: usize = size_of::<SizeCache<42>>();
const NODE_SIZE: usize = size_of::<Node<NonNull<dyn CacheBase>>>();

pub struct CacheManager<'a> {
	cache_space: SizeCache<'a, CACHE_ALLOCATOR_SIZE>,
	node_space: SizeCache<'a, NODE_SIZE>,
	list: NAList<NonNull<dyn CacheBase>>,
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
		A: CacheBase + CacheInit + 'static, // TODO why static?
	{
		let mem_cache = self.cache_space.alloc()?.as_ptr() as *mut A;
		let mem_node = self.node_space.alloc()?.as_ptr();

		unsafe {
			A::cache_init(mem_cache);
			let ptr_cache = mem_cache as *mut dyn CacheBase;
			let node = init_list_node(mem_node, ptr_cache);
			self.list.push_front(node);
		}
		NonNull::new(mem_cache).ok_or(AllocError)
	}

	pub fn register(&mut self, cache: &'static mut dyn CacheBase) -> Result<()> {
		// TODO why static?
		let mem_node = self.node_space.alloc()?.as_ptr();
		let node = unsafe { init_list_node(mem_node, cache as *mut dyn CacheBase) };

		self.list.push_front(node);
		Ok(())
	}

	/// Safety
	///
	/// `ptr` must point cache alloctor.
	pub unsafe fn drop_allocator<A>(&mut self, ptr: NonNull<A>)
	where
		A: CacheBase + 'static,
	{
		let cache = &mut *(ptr.as_ptr() as *mut dyn CacheBase);
		cache.cache_shrink();
		if !cache.empty() {
			panic!("It can cause memory leak!");
		}

		let cache_ptr = self.list.remove_if(|n| n.as_ref() == cache);
		cache_ptr.map(|mut cache_ptr| {
			let ptr = cache_ptr.as_mut();
			self.node_space.dealloc(ptr.cast());
			self.cache_space.dealloc(ptr.cast());
		});
	}

	pub fn unregister(&mut self, cache: &'static mut dyn CacheBase) {
		unsafe {
			let cache_ptr = self.list.remove_if(|n| n.as_ref() == cache);
			cache_ptr.map(|mut cache_ptr| {
				let ptr = cache_ptr.as_mut();
				self.node_space.dealloc(ptr.cast());
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
	cache: *mut dyn CacheBase,
) -> &'a mut Node<NonNull<dyn CacheBase>> {
	let data = NonNull::new_unchecked(cache);
	let ptr = NonNull::new_unchecked(mem_node);
	Node::construct_at(ptr, data)
}

#[macro_export]
macro_rules! kmem_cache_register {
	($cache:ident) => {
		let mut err_count = 0;
		for _ in 0..$crate::mm::slub::REGISTER_TRY {
			match $crate::mm::slub::CM.register(&mut $cache) {
				Ok(_) => break,
				Err(_) => {
					// pr_debug;
					err_count += 1;
					$crate::mm::slub::CM.cache_shrink();
				}
			}
		}
		if err_count == $crate::mm::slub::REGISTER_TRY {
			$crate::pr_info!("cache_manager: register: out of memory.");
			panic!(); // TODO 이게 맞나..?
		}
	};
}

mod tests {
	use kfs_macro::ktest;

	use crate::mm::slub::{
		cache::{align_with_hw_cache, CacheBase},
		size_cache::SizeCache,
	};

	use super::super::{
		cache_manager::{CACHE_ALLOCATOR_SIZE, NODE_SIZE},
		PAGE_SIZE,
	};
	use super::CacheManager;

	// fn check_remains<'a, const N: usize>(space: &mut SizeCache<'a, N>, alloc_size: usize) {
	// 	let mut head_ptr = space.partial().head().unwrap();
	// 	let head = unsafe { head_ptr.as_mut() };

	// 	let offset = if alloc_size == 0 {
	// 		0
	// 	} else {
	// 		align_with_hw_cache(alloc_size)
	// 	};

	// 	assert_eq!(head.bytes(), PAGE_SIZE - offset);
	// }

	// #[ktest]
	// fn test_cache_alloc_dealloc() {
	// 	let mut cm = CacheManager::new();
	// 	let ptr = cm.new_allocator::<SizeCache<2048>>().unwrap();
	// 	let _ = unsafe { &mut *(ptr.as_ptr()) };

	// 	check_remains(&mut cm.cache_space, CACHE_ALLOCATOR_SIZE);
	// 	check_remains(&mut cm.node_space, NODE_SIZE);
	// 	assert_eq!(1, cm.list.count());

	// 	unsafe { cm.drop_allocator(ptr) };

	// 	check_remains(&mut cm.cache_space, 0);
	// 	check_remains(&mut cm.node_space, 0);
	// 	assert_eq!(0, cm.list.count());
	// }

	// static mut SIZE_CACHE: SizeCache<1024> = SizeCache::new();

	// #[ktest]
	// fn test_register_unregister() {
	// 	let mut cm = CacheManager::new();

	// 	cm.register(unsafe { &mut SIZE_CACHE }).unwrap();

	// 	check_remains(&mut cm.node_space, NODE_SIZE);
	// 	assert_eq!(1, cm.list.count());

	// 	cm.unregister(unsafe { &mut SIZE_CACHE });

	// 	check_remains(&mut cm.node_space, 0);
	// 	assert_eq!(0, cm.list.count());
	// }
}
