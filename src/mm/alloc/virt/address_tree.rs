use alloc::collections::BTreeSet;
use core::ops::{
	Bound::{Excluded, Included, Unbounded},
	Range,
};

use crate::mm::util::*;

use super::{OrdByCount, OrdByPfn, Page};

/// Maintain size and begin address of pages in `area`
pub struct AddressTree {
	area: Range<usize>,
	by_count: BTreeSet<OrdByCount>,
	by_pfn: BTreeSet<OrdByPfn>,
}

impl AddressTree {
	pub fn new(area: Range<usize>) -> Self {
		let mut by_count = BTreeSet::new();
		let mut by_pfn = BTreeSet::new();

		let page = Page::new(area.start, area.end - area.start);
		by_count.insert(OrdByCount(page));
		by_pfn.insert(OrdByPfn(page));

		Self {
			area,
			by_count,
			by_pfn,
		}
	}

	/// Check `count` of page is available.
	/// if so, update metadata and return begining address.
	pub fn alloc(&mut self, count: usize) -> Option<usize> {
		let page = self
			.by_count
			.range((Included(OrdByCount(Page::from_count(count))), Unbounded))
			.next()?
			.0;

		self.by_count.remove(&OrdByCount(page));
		self.by_pfn.remove(&OrdByPfn(page));

		let new_page = Page::new(page.pfn + count, page.count - count);

		self.by_count.insert(new_page.into());
		self.by_pfn.insert(new_page.into());

		Some(pfn_to_addr(page.pfn))
	}

	/// deallocate page and update metadata.
	pub fn dealloc(&mut self, addr: usize, count: usize) {
		let page = Page::new(addr_to_pfn(addr), count);

		let lower = self
			.by_pfn
			.range((Unbounded, Excluded(OrdByPfn(page))))
			.next_back()
			.map(|x| x.0.clone())
			.and_then(|x| {
				if x.end_pfn() == page.pfn {
					self.by_pfn.remove(&OrdByPfn(x));
					self.by_count.remove(&OrdByCount(x));

					Some(x.pfn)
				} else {
					None
				}
			})
			.unwrap_or_else(|| page.pfn);

		let upper = self
			.by_pfn
			.range((Excluded(OrdByPfn(page)), Unbounded))
			.next()
			.map(|x| x.0.clone())
			.and_then(|x| {
				if x.pfn == page.end_pfn() {
					self.by_pfn.remove(&OrdByPfn(x));
					self.by_count.remove(&OrdByCount(x));

					Some(x.end_pfn())
				} else {
					None
				}
			})
			.unwrap_or_else(|| page.end_pfn());

		let page = Page::new(lower, upper - lower);

		self.by_pfn.insert(OrdByPfn(page));
		self.by_count.insert(OrdByCount(page));
	}
}
