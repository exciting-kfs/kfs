use core::ops::Range;

use alloc::vec::Vec;

use crate::{
	driver::ide::{block::Block, lba::LBA28},
	mm::constant::{KB, SECTOR_SIZE},
};

use super::{
	call_back::{CallBack, Cleanup},
	event::DmaInit,
};

pub struct ReqInit {
	pub(super) range: Range<LBA28>,
	pub(super) cb: CallBack,
}

impl ReqInit {
	pub const MAX_KB: usize = 128;

	pub fn new(range: Range<LBA28>, cb: CallBack) -> Self {
		Self { range, cb }
	}

	pub(super) fn can_merge(&self, req: &Self) -> bool {
		(self.kilo_bytes() + req.kilo_bytes() <= DmaInit::MAX_KB)
			&& (self.range.start == req.range.end || self.range.end == req.range.start)
	}

	pub(super) fn merge(&mut self, event: Self) {
		let Self { range: rng, cb } = event;

		self.cb.merge(cb);

		self.range = match self.range.end < rng.end {
			true => self.range.start..rng.end,
			false => rng.start..self.range.end,
		}
	}

	fn kilo_bytes(&self) -> usize {
		(self.range.end - self.range.start) * SECTOR_SIZE / KB
	}
}

pub struct ReqReady {
	pub(super) range: Range<LBA28>,
	pub(super) blocks: Vec<Block>,
	pub(super) cleanup: Vec<Cleanup>,
}

impl ReqReady {
	pub fn count(&self) -> usize {
		self.range.end - self.range.start
	}
}
