//! Buddy allocator

use super::free_list::FreeList;

use crate::mm::page::{index_to_meta, meta_to_index, meta_to_unmapped, phys_to_meta, MetaPage};
use crate::mm::{constant::*, util::*};
use crate::ptr::UnMapped;

use core::alloc::AllocError;
use core::fmt::{self, Display};

use core::ops::Range;
use core::ptr::{addr_of_mut, NonNull};

pub struct BuddyAlloc {
	free_list: FreeList,
}

impl BuddyAlloc {
	/// Construct new buddy allocator which covers `cover_pfn` at pointed by `ptr`
	pub unsafe fn construct_at(ptr: *mut BuddyAlloc, mut cover_pfn: Range<usize>) {
		let free_list = FreeList::construct_at(addr_of_mut!((*ptr).free_list));

		cover_pfn.start = next_align(cover_pfn.start, rank_to_pages(MAX_RANK));
		cover_pfn.end = cover_pfn.end & !(rank_to_pages(MAX_RANK) - 1);

		for phys_pfn in cover_pfn.step_by(rank_to_pages(MAX_RANK)) {
			// ignore zero page
			if pfn_phys_to_virt(phys_pfn) == 0 {
				continue;
			}
			let mut entry = index_to_meta(phys_pfn);
			entry.as_mut().set_rank(MAX_RANK);
			free_list.add(entry);
		}
	}

	/// allocate `2 ^ req_rank` of pages.
	pub fn alloc_pages(&mut self, req_rank: usize) -> Result<UnMapped, AllocError> {
		for rank in req_rank..=MAX_RANK {
			if let Some(page) = self.free_list.get(rank) {
				return Ok(self.split_to_rank(page, req_rank));
			}
		}
		return Err(AllocError);
	}

	/// deallocate pages.
	pub fn free_pages(&mut self, ptr: UnMapped) {
		let mut page = phys_to_meta(ptr.as_phys());
		unsafe { page.as_mut().set_inuse(false) };

		while let Some(mut buddy) = self.get_free_buddy(page) {
			unsafe { buddy.as_mut().disjoint() };
			page = unsafe { page.as_mut().merge(buddy) };
		}

		self.free_list.add(page);
	}

	fn split_to_rank(&mut self, page: NonNull<MetaPage>, req_rank: usize) -> UnMapped {
		let mut lpage = page;
		let mut rpage;
		while req_rank < unsafe { lpage.as_mut().rank() } {
			(lpage, rpage) = unsafe { lpage.as_mut().split() };
			self.free_list.add(rpage);
		}

		unsafe { lpage.as_mut().set_inuse(true) };

		meta_to_unmapped(lpage)
	}

	fn get_free_buddy(&mut self, page: NonNull<MetaPage>) -> Option<NonNull<MetaPage>> {
		let rank = unsafe { page.as_ref().rank() };

		if rank >= MAX_RANK {
			return None;
		}

		let buddy_index = meta_to_index(page) ^ rank_to_pages(rank);
		let buddy_page = unsafe { index_to_meta(buddy_index).as_ref() };

		return (!buddy_page.inuse() && unsafe { page.as_ref().rank() } == buddy_page.rank())
			.then(|| NonNull::from(buddy_page));
	}
}

impl Display for BuddyAlloc {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.free_list)
	}
}
