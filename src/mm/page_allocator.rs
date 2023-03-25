//! Kernel page allocator.
//! Current implementation is Buddy-allocator.
//!
//! # Terms used in implementation.
//!
//! - node: Unit of each allocation. each node holds up to `2 ^ MAX_RANK` pages.
//!
//! - page: Continous, `PAGE_SIZE` aligned, `PAGE_SIZE` sized space in memory.
//!
//! - block: Continous `2 ^ MAX_RANK` pages. simularly it's `2 ^ MAX_RANK * PAGE_SIZE` aligned.
//!
//! # Implementation detail
//!
//! After buddy_init() call memory layout(physical) will be
//! `[ KERNEL MATAPAGES BLOCK BLOCK BLOCK ... ]`
//!
//! #
//!
//!
//!
//!  
//! To get better result in term of speed,
//! We devided big continous memory into `blocks`.
//! That way, we can limit maximum (split, merge) operation per request by `MAX_RANK`.
//! But, It also reduces maximum allocation size per request by `1 block`.

pub use buddy_allocator::Page;
pub mod util;

mod buddy_allocator;
mod constant;
mod free_list;

use super::{meta_page::init_meta_page_table, util::current_or_next_aligned, x86::init::ZoneInfo};
use buddy_allocator::BuddyAllocator;
use core::{
	mem::{align_of, size_of},
	ptr::NonNull,
	slice::from_raw_parts_mut,
};

pub struct PageAllocator {
	high: BuddyAllocator,
	normal: BuddyAllocator,
}

impl PageAllocator {
	pub unsafe fn new(zone_info: &mut ZoneInfo) -> &'static mut Self {
		let page_alloc_start = current_or_next_aligned(zone_info.normal.start, align_of::<Self>());
		zone_info.normal.start = page_alloc_start + size_of::<Self>();

		let meta_page_table = init_meta_page_table(zone_info);

		let page_alloc = page_alloc_start as *mut Self;

		BuddyAllocator::construct_at(
			(&mut (*page_alloc).normal) as *mut BuddyAllocator,
			zone_info.normal.clone(),
			from_raw_parts_mut(meta_page_table.as_mut_ptr(), meta_page_table.len()),
		);

		BuddyAllocator::construct_at(
			(&mut (*page_alloc).high) as *mut BuddyAllocator,
			zone_info.high.clone(),
			from_raw_parts_mut(meta_page_table.as_mut_ptr(), meta_page_table.len()),
		);

		return &mut *page_alloc;
	}

	pub fn alloc_page(&mut self, rank: usize) -> Result<NonNull<Page>, ()> {
		self.normal.alloc_page(rank)
	}

	pub fn free_page(&mut self, page: NonNull<Page>) {
		self.normal.free_page(page);
	}
}
