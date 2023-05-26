use crate::mm::{constant::*, util::*};
use crate::util::bitrange::BitRange;

#[repr(transparent)]
pub struct MetaData(usize);

impl MetaData {
	pub const INUSE: BitRange = BitRange::new(0, 1);
	pub const RANK: BitRange = BitRange::new(1, 5);
	const UNUSED_AREA: BitRange = BitRange::new(5, PAGE_SHIFT);
	const MAPPED_ADDR: BitRange = BitRange::new(PAGE_SHIFT, usize::BITS as usize);

	pub fn new_unmapped() -> Self {
		MetaData(0)
	}

	pub fn new(mapped_addr: usize) -> Self {
		MetaData(mapped_addr & Self::MAPPED_ADDR.mask())
	}

	pub fn remap(&mut self, new_addr: usize) {
		self.0 = (self.0 & !Self::MAPPED_ADDR.mask()) | (new_addr & Self::MAPPED_ADDR.mask());
	}

	pub fn mapped_addr(&self) -> usize {
		self.0 & Self::MAPPED_ADDR.mask()
	}

	pub fn get_flag(&self, range: BitRange) -> usize {
		(self.0 & range.mask()) >> range.start
	}

	pub fn set_flag(&mut self, range: BitRange, bits: usize) {
		self.0 = (self.0 & !range.mask()) | ((bits << range.start) & range.mask())
	}
}

mod test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	pub fn rank() {
		let mut data = MetaData::new_unmapped();

		for i in 0..=10 {
			data.set_flag(MetaData::RANK, i);
			assert!(data.get_flag(MetaData::RANK) == i);
		}
	}

	#[ktest]
	pub fn new() {
		let addr = pfn_to_addr(42);

		let data = MetaData::new(addr);
		let new_addr = data.mapped_addr();

		assert!(addr == new_addr);
	}

	#[ktest]
	pub fn remap() {
		let mut data = MetaData::new_unmapped();
		assert!(data.mapped_addr() == 0);

		let addr = pfn_to_addr(42);
		data.remap(addr);
		assert!(data.mapped_addr() == addr);
	}
}
