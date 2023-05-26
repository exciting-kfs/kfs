use core::cmp::Ordering;

/// Represent `count` of continuous pages.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct Page {
	pub pfn: usize,
	pub count: usize,
}

impl Page {
	pub fn new(pfn: usize, count: usize) -> Self {
		Self { pfn, count }
	}

	pub fn from_pfn(pfn: usize) -> Self {
		Self { pfn, count: 0 }
	}

	pub fn from_count(count: usize) -> Self {
		Self { pfn: 0, count }
	}

	pub fn end_pfn(&self) -> usize {
		self.pfn + self.count
	}
}

/// Page but ordered by PFN.
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct OrdByPfn(pub Page);

impl From<Page> for OrdByPfn {
	fn from(value: Page) -> Self {
		Self(value)
	}
}

impl PartialOrd for OrdByPfn {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match self.0.pfn.partial_cmp(&other.0.pfn) {
			Some(Ordering::Equal) => self.0.count.partial_cmp(&other.0.count),
			x => x,
		}
	}
}

impl Ord for OrdByPfn {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		match self.0.pfn.cmp(&other.0.pfn) {
			Ordering::Equal => self.0.count.cmp(&other.0.count),
			x => x,
		}
	}
}

/// Page but ordered by count(size).
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct OrdByCount(pub Page);

impl From<Page> for OrdByCount {
	fn from(value: Page) -> Self {
		Self(value)
	}
}

impl PartialOrd for OrdByCount {
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
		match self.0.count.partial_cmp(&other.0.count) {
			Some(Ordering::Equal) => self.0.pfn.partial_cmp(&other.0.pfn),
			x => x,
		}
	}
}

impl Ord for OrdByCount {
	fn cmp(&self, other: &Self) -> core::cmp::Ordering {
		match self.0.count.cmp(&other.0.count) {
			Ordering::Equal => self.0.pfn.cmp(&other.0.pfn),
			x => x,
		}
	}
}
