mod utils;
mod obj_cache;
mod size_cache;
mod cache_manager;

pub use size_cache::SizeCache;
pub use size_cache::ForSizeCache;
pub use cache_manager::CM;
pub use cache_manager::REGISTER_TRY;
pub use utils::{bit_scan_forward, bit_scan_reverse};

use self::utils::Error;
use self::utils::free_list::FreeList;

use super::PAGE_SIZE;
use super::alloc_pages_from_buddy;
use super::dealloc_pages_to_buddy;

pub trait CacheBase {
	fn free_list(&mut self) -> &mut FreeList;
	fn page_count(&mut self) -> &mut usize;
}

pub trait CacheShrink: CacheBase {
	fn cache_shrink(&mut self) {
		let free_list = self.free_list();
		let (mut satisfied, not) = free_list
			.iter_mut()
			.partition(|node| node.bytes() >= PAGE_SIZE);

		(*free_list) = not;

		satisfied.iter_mut().for_each(|node| {
			let (ptr, count, bot) = node.shrink();
			if let Some(new_node) = bot {
				free_list.insert(new_node);
			}
			if node.bytes() > 0 {
				free_list.insert(node);
			}
			unsafe { dealloc_pages_to_buddy(ptr, count) };
		});
	}
}

trait PageAlloc<'page>: CacheBase {
	fn alloc_pages(&mut self, count: usize) -> Result<&'page mut [u8], Error> {
		let page = alloc_pages_from_buddy::<'page>(count).ok_or(Error::Alloc)?;
		let page_count = self.page_count();
		(*page_count) += count;
		Ok(page)
	}

	fn dealloc_pages(&mut self, ptr: *mut u8, count: usize) {
		unsafe { dealloc_pages_to_buddy(ptr, count) };
		let page_count = self.page_count();
		(*page_count) -= count;
	}
}

impl PartialEq for dyn CacheShrink {
	fn eq(&self, other: &Self) -> bool {
	    self as *const dyn CacheShrink as *const u8 == other as *const dyn CacheShrink as *const u8
	}
}


pub trait CacheInit: Default {
	unsafe fn cache_init(ptr: *mut Self) {
		(*ptr) = Self::default();
	}
}

