use crate::util::bitrange::{BitData, BitRange};

#[repr(transparent)]
pub struct MetaData(BitData);

impl MetaData {
	pub const INUSE: BitRange = BitRange::new(0, 28);
	pub const RANK: BitRange = BitRange::new(28, 32);

	pub fn new() -> Self {
		MetaData(BitData::new(0))
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
		let mut data = MetaData::new();

		for i in 0..=10 {
			data.set_flag(MetaData::RANK, i);
			assert!(data.get_flag(MetaData::RANK) == i);
		}
	}
}
