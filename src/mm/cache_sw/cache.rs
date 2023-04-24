use super::PAGE_SIZE;
use super::dealloc_pages_to_buddy;
use super::size_cache::free_list::FreeList;

pub trait CacheBase {
	fn free_list(&mut self) -> &mut FreeList;
	fn inuse(&self) -> usize;
}

pub trait CacheShrink: CacheBase {
	fn cache_shrink(&mut self) {
		let free_list = self.free_list();
		let (mut satisfied, not) = free_list
			.iter_mut()
			.partition(|node| node.bytes() >= PAGE_SIZE);

		(*free_list) = not;

		satisfied.iter_mut().for_each(|node| {
			let (page_ptr, page_count, extra) = node.shrink();
			if let Some(new_node) = extra {
				free_list.insert(new_node);
			}
			if node.bytes() > 0 {
				free_list.insert(node);
			}
			unsafe { dealloc_pages_to_buddy(page_ptr, page_count) };
		});
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

pub const fn align_with_hw_cache(bytes: usize) -> usize {
	const CACHE_LINE_SIZE : usize = 64; // L1

	match bytes {
		0..=16 => 16,
		17..=32 => 32,
		_ => CACHE_LINE_SIZE * ((bytes - 1) / CACHE_LINE_SIZE + 1)
	}
}