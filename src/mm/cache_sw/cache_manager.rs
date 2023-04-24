mod no_alloc_list;

use core::mem::size_of;
use core::ptr::NonNull;
use core::alloc::AllocError;

use self::no_alloc_list::{Node, NAList};

use super::{cache::{CacheShrink, CacheInit}, size_cache::SizeCache};

type Result<T> = core::result::Result<T, AllocError>;

pub static mut CM: CacheManager<'static> = CacheManager::new();

const CACHE_ALLOCATOR_SIZE: usize = size_of::<SizeCache<42>>();
const NODE_SIZE: usize = size_of::<Node::<NonNull<dyn CacheShrink>>>();

pub struct CacheManager<'a> {
	cache_space: SizeCache<'a, CACHE_ALLOCATOR_SIZE>,
	node_space: SizeCache<'a, NODE_SIZE>,
	list: NAList<NonNull<dyn CacheShrink>>,
}

impl<'a> CacheManager<'a> {
	pub const fn new() -> Self {
		CacheManager { cache_space: SizeCache::new(), node_space: SizeCache::new(), list: NAList::new() }
	}

	pub fn new_allocator<A>(&mut self) -> Result<*mut A>
	where A: CacheShrink + CacheInit + 'static // TODO why static?
	{
		let for_cache =  self.cache_space.alloc()? as *mut A;
		unsafe { A::cache_init(for_cache) };

		let for_node = self.node_space.alloc()?;
		let node = unsafe {
			let node_ptr = core::slice::from_raw_parts_mut(for_node, NODE_SIZE);
			let data = NonNull::new_unchecked(for_cache as *mut dyn CacheShrink);
			Node::construct_at(node_ptr, data)
		};

		self.list.insert_front(node);
		Ok(for_cache.cast())
	}

	pub fn register(&mut self, cache: &'static mut dyn CacheShrink) -> Result<()> { // TODO why static?
		let node = unsafe {
			let data = NonNull::new_unchecked(cache as *mut dyn CacheShrink);
			let for_node = self.node_space.alloc()?;

			let node_ptr = core::slice::from_raw_parts_mut(for_node, NODE_SIZE);
			Node::construct_at(node_ptr, data)
		};

		self.list.insert_front(node);
		Ok(())
	}

	pub unsafe fn drop_allocator<A>(&mut self, ptr: *mut A)
	where A: CacheShrink + 'static
	{
		let cache = &mut *(ptr as *mut dyn CacheShrink);
		cache.cache_shrink();
		if cache.inuse() != 0 {
			panic!("It can cause memory leak!");
		}
		
		let node = self.list.remove_if(|n| n.data().as_ref() == cache );
		node.map(|node| {
			self.node_space.dealloc(node.as_mut_ptr().cast());
			self.cache_space.dealloc(ptr as *mut u8);
		});
	}

	pub fn unregister(&mut self, cache: &'static mut dyn CacheShrink) {
		unsafe {
			let node = self.list.remove_if(|n| n.data().as_ref() == cache );
			node.map(|node| {
				self.node_space.dealloc(node.as_mut_ptr().cast());
			});
		}
	}

	pub fn cache_shrink(&mut self) {
		self.cache_space.cache_shrink();
		self.node_space.cache_shrink();
		self.list.iter_mut().for_each(|node| {
			let cache = unsafe { node.data_mut().as_mut() };
			cache.cache_shrink();
		})
	}
}


#[macro_export]
macro_rules! kmem_cache_register {
	($cache:ident) => {
		for _ in 0..$crate::mm::cache_sw::REGISTER_TRY {
			match $crate::mm::cache_sw::CM.register(&mut $cache) {
				Ok(_) => break,
				Err(_) => {
					// pr_debug;
					$crate::mm::cache_sw::CM.cache_shrink();
				},
			}
		}
	}
}

mod tests {
	use kfs_macro::ktest;

	use crate::{mm::cache_sw::{
		cache::{align_with_hw_cache, CacheBase},
		size_cache::SizeCache
	}};

	use super::CacheManager;
	use super::super::{
		PAGE_SIZE,
		cache_manager::{CACHE_ALLOCATOR_SIZE, NODE_SIZE}
	};
 	
	fn check_remains<'a, const N: usize>(space: &mut SizeCache<'a, N>, alloc_size: usize) {
		let mut head_ptr = space.free_list().head().unwrap();
		let head = unsafe { head_ptr.as_mut() };

		let offset = if alloc_size == 0 {
			0
		} else {
			align_with_hw_cache(alloc_size)
		};

		assert_eq!(head.bytes(), PAGE_SIZE - offset);
	}

	#[ktest]
	fn test_cache_alloc_dealloc() {		
		let mut cm = CacheManager::new();
		let cache = unsafe { &mut *cm.new_allocator::<SizeCache<2048>>().unwrap() };

		check_remains(&mut cm.cache_space, CACHE_ALLOCATOR_SIZE);
		check_remains(&mut cm.node_space, NODE_SIZE);
		assert_eq!(1, cm.list.count());

		unsafe { cm.drop_allocator(cache as *mut SizeCache<2048>) };
		
		check_remains(&mut cm.cache_space, 0);
		check_remains(&mut cm.node_space, 0);
		assert_eq!(0, cm.list.count());
	}

	static mut SIZE_CACHE : SizeCache<1024> = SizeCache::new();

	#[ktest]
	fn test_register_unregister() {
		let mut cm = CacheManager::new();

		cm.register(unsafe { &mut SIZE_CACHE }).unwrap();

		check_remains(&mut cm.node_space, NODE_SIZE);
		assert_eq!(1, cm.list.count());

		cm.unregister(unsafe { &mut SIZE_CACHE });

		check_remains(&mut cm.node_space, 0);
		assert_eq!(0, cm.list.count());
	}
}
