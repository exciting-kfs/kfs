use alloc::vec::Vec;

use crate::{
	driver::ide::{block::Block, lba::LBA28},
	mm::constant::{KB, SECTOR_SIZE},
};

use super::event::CallBack;

pub struct WriteDma {
	pub(super) begin: LBA28,
	pub(super) end: LBA28,
	pub(super) own: Vec<Block>,
	pub(super) cb: CallBack,
}

impl WriteDma {
	pub const MAX_KB: usize = 128;

	pub fn new(begin: LBA28, end: LBA28, cb: CallBack) -> Self {
		Self {
			begin,
			end,
			own: Vec::new(),
			cb,
		}
	}

	pub fn count(&self) -> usize {
		self.end - self.begin
	}

	pub fn kilo_bytes(&self) -> usize {
		self.count() * SECTOR_SIZE / KB
	}

	pub fn merge(&mut self, event: Self) {
		let Self {
			begin,
			end,
			own: _,
			cb,
		} = event;

		self.cb.merge(cb);

		if self.begin > begin {
			self.begin = begin
		}

		if self.end < end {
			self.end = end
		}
	}
}
