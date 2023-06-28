use crate::mm::{constant::*, util::*};
use crate::util::bitrange::{BitData, BitRange};

#[repr(transparent)]
pub struct MetaData(BitData);

impl MetaData {
	pub const INUSE: BitRange = BitRange::new(0, 1);
	pub const RANK: BitRange = BitRange::new(1, 5);
	const UNUSED_AREA: BitRange = BitRange::new(5, PAGE_SHIFT);
	const MAPPED_ADDR: BitRange = BitRange::new(PAGE_SHIFT, usize::BITS as usize);

	pub fn new_unmapped() -> Self {
		MetaData(BitData::new(0))
	}

	pub fn new(mapped_addr: usize) -> Self {
		MetaData(BitData::new(mapped_addr & Self::MAPPED_ADDR.mask()))
	}

	pub fn remap(&mut self, new_addr: usize) {
		self.0
			.erase_bits(&Self::MAPPED_ADDR)
			.add_bits(&Self::MAPPED_ADDR, new_addr);
	}

	pub fn mapped_addr(&self) -> usize {
		self.0.get_bits(&Self::MAPPED_ADDR)
	}

	pub fn get_flag(&self, range: BitRange) -> usize {
		self.0.shift_get_bits(&range)
	}

	pub fn set_flag(&mut self, range: BitRange, bits: usize) {
		self.0.erase_bits(&range).shift_add_bits(&range, bits);
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
