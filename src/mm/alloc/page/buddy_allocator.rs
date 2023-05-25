//! Another Buddy allocator implementation.
//! Welcome to the WILD.

use super::free_list::FreeList;

use crate::mm::page::{MetaPage, MetaPageTable, META_PAGE_TABLE};
use crate::mm::{constant::*, util::*};

use core::alloc::AllocError;
use core::fmt::{self, Display};

use core::ops::Range;
use core::ptr::{addr_of_mut, NonNull};
use core::slice::from_raw_parts;

pub struct BuddyAlloc {
	free_list: FreeList,
}

impl BuddyAlloc {
	pub unsafe fn construct_at(ptr: *mut BuddyAlloc, mut cover_pfn: Range<usize>) {
		let free_list = FreeList::construct_at(addr_of_mut!((*ptr).free_list));

		cover_pfn.start = next_align(cover_pfn.start, rank_to_pages(MAX_RANK));

		for mut entry in cover_pfn
			.step_by(rank_to_pages(MAX_RANK))
			.map(|virt_pfn| addr_to_pfn(virt_to_phys(pfn_to_addr(virt_pfn))))
			.map(|phys_pfn| NonNull::from(&mut META_PAGE_TABLE.lock()[phys_pfn]))
		{
			entry.as_mut().set_rank(MAX_RANK);
			free_list.add(entry);
		}
	}

	pub fn alloc_page(&mut self, req_rank: usize) -> Result<NonNull<[u8]>, AllocError> {
		for rank in req_rank..=MAX_RANK {
			if let Some(page) = self.free_list.get(rank) {
				return Ok(self.split_to_rank(page, req_rank));
			}
		}
		return Err(AllocError);
	}

	pub fn free_page(&mut self, ptr: NonNull<u8>) {
		let mut page = MetaPageTable::ptr_to_metapage(ptr);
		unsafe { page.as_mut().set_inuse(false) };

		while let Some(mut buddy) = self.get_free_buddy(page) {
			unsafe { buddy.as_mut().disjoint() };
			page = unsafe { page.as_mut().merge(buddy) };
		}

		self.free_list.add(page);
	}

	fn split_to_rank(&mut self, page: NonNull<MetaPage>, req_rank: usize) -> NonNull<[u8]> {
		let mut lpage = page;
		let mut rpage;
		while req_rank < unsafe { lpage.as_mut().rank() } {
			(lpage, rpage) = unsafe { lpage.as_mut().split() };
			self.free_list.add(rpage);
		}

		unsafe { lpage.as_mut().set_inuse(true) };

		let page = MetaPageTable::metapage_to_ptr(lpage);
		unsafe { NonNull::from(from_raw_parts(page.as_ptr(), rank_to_size(req_rank))) }
	}

	fn get_free_buddy(&mut self, page: NonNull<MetaPage>) -> Option<NonNull<MetaPage>> {
		let rank = unsafe { page.as_ref().rank() };

		if rank >= MAX_RANK {
			return None;
		}

		let buddy_index = MetaPageTable::metapage_to_index(page) ^ rank_to_pages(rank);
		let buddy_page = unsafe { MetaPageTable::index_to_metapage(buddy_index).as_ref() };

		return (!buddy_page.inuse() && unsafe { page.as_ref().rank() } == buddy_page.rank())
			.then(|| NonNull::from(buddy_page));
	}
}

impl Display for BuddyAlloc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.free_list)
	}
}
