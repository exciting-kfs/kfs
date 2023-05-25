use core::ptr::NonNull;

use crate::mm::page::MetaPage;
use crate::mm::{constant::*, util::*};

#[repr(transparent)]
pub struct FreeList {
	list: [MetaPage; MAX_RANK + 1],
}

/// First block's PFN.
/// used to lookup metapage by address.
// static mut FIRST_PFN: usize = 0;
// static mut METADATA: *mut MetaPage = null_mut();
// static mut FREE_LIST: FreeList = FreeList::new();

impl FreeList {
	pub unsafe fn construct_at(ptr: *mut FreeList) -> &'static mut FreeList {
		for entry in &mut (*ptr).list {
			MetaPage::construct_at(entry as *mut MetaPage);
		}

		return &mut *ptr;
	}

	pub fn add(&mut self, page: NonNull<MetaPage>) {
		self.list[unsafe { page.as_ref().rank() }].push(page);
	}

	pub fn get(&mut self, rank: usize) -> Option<NonNull<MetaPage>> {
		match rank <= MAX_RANK {
			true => self.list[rank].pop(),
			false => None,
		}
	}
}

use core::fmt::{self, Display};

impl Display for FreeList {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let mut total_pages = 0;
		for (rank, head) in self.list.iter().enumerate() {
			let nodes = (&head).into_iter().count();
			writeln!(
				f,
				"[R{:02}]: {:03} Node ({} PAGE)",
				rank,
				nodes,
				rank_to_pages(rank) * nodes,
			)?;

			total_pages += rank_to_pages(rank) * nodes;
		}
		write!(f, "= TOTAL {} FREE PAGES. =", total_pages)
	}
}
