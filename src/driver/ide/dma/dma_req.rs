use core::ops::Range;

use alloc::vec::Vec;

use crate::{
	driver::ide::{block::Block, lba::LBA28},
	mm::constant::{KB, SECTOR_SIZE},
};

use super::{
	event::DmaInit,
	hook::{Cleanup, CleanupWB, ItemWB, OwnHook, WBHook},
};

pub struct ReqInit {
	pub(super) range: Range<LBA28>,
	pub(super) cb: OwnHook,
}

impl ReqInit {
	pub const MAX_KB: usize = 128;

	pub fn new(range: Range<LBA28>, cb: OwnHook) -> Self {
		Self { range, cb }
	}

	pub(super) fn can_merge(&self, req: &Self) -> bool {
		(self.kilo_bytes() + req.kilo_bytes() <= DmaInit::MAX_KB)
			&& (self.range.start == req.range.end || self.range.end == req.range.start)
	}

	pub(super) fn merge(&mut self, event: &mut Self) {
		self.cb.merge(&mut event.cb);

		self.range = match self.range.end < event.range.end {
			true => self.range.start..event.range.end,
			false => event.range.start..self.range.end,
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

pub struct ReqWBInit {
	pub(super) range: Range<LBA28>,
	pub(super) cb: WBHook,
}

impl ReqWBInit {
	pub const MAX_KB: usize = 128;

	pub fn new(range: Range<LBA28>, cb: WBHook) -> Self {
		Self { range, cb }
	}

	pub(super) fn can_merge(&self, req: &Self) -> bool {
		(self.kilo_bytes() + req.kilo_bytes() <= DmaInit::MAX_KB)
			&& (self.range.start == req.range.end || self.range.end == req.range.start)
	}

	pub(super) fn merge(&mut self, event: &mut Self) {
		self.cb.merge(&mut event.cb);

		self.range = match self.range.end < event.range.end {
			true => self.range.start..event.range.end,
			false => event.range.start..self.range.end,
		}
	}

	fn kilo_bytes(&self) -> usize {
		(self.range.end - self.range.start) * SECTOR_SIZE / KB
	}
}

pub struct ReqWBReady {
	pub(super) range: Range<LBA28>,
	pub(super) blocks: Vec<ItemWB>,
	pub(super) cleanup: Vec<CleanupWB>,
}
