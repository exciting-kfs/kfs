use core::{alloc::AllocError, mem::take};

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use crate::{
	driver::ide::{block::Block, lba::LBA28, IdeController},
	mm::constant::{KB, SECTOR_SIZE},
	pr_debug,
	sync::locked::LockedGuard,
};

use super::DmaOps;

#[derive(Default)]
pub struct CallBack {
	pub prologue: BTreeMap<LBA28, Box<dyn FnOnce() -> Result<Block, AllocError>>>,
	pub epilogue: BTreeMap<LBA28, Box<dyn FnOnce(Block) -> ()>>,
}

impl CallBack {
	pub fn new() -> Self {
		Self {
			prologue: BTreeMap::new(),
			epilogue: BTreeMap::new(),
		}
	}

	fn merge(&mut self, cb: Self) {
		let Self {
			mut prologue,
			mut epilogue,
		} = cb;
		self.prologue.append(&mut prologue);
		self.epilogue.append(&mut epilogue);
	}
}

pub struct Event {
	pub(super) kind: DmaOps,
	pub(super) begin: LBA28,
	pub(super) end: LBA28,
	own: Vec<Block>,
	cb: CallBack,
}

impl Event {
	pub const MAX_KB: usize = 128;

	pub fn new(kind: DmaOps, begin: LBA28, end: LBA28, cb: CallBack) -> Self {
		Self {
			kind,
			begin,
			end,
			own: Vec::new(),
			cb,
		}
	}

	pub fn prepare(&mut self) -> Result<Vec<Block>, AllocError> {
		let callbacks = take(&mut self.cb.prologue);
		let results = callbacks.into_iter().map(|(_, cb)| cb());
		let mut blocks = Vec::new();

		for b in results {
			blocks.push(b?);
		}
		Ok(blocks)
	}

	pub fn cleanup(self) {
		let Self {
			kind: _,
			begin: _,
			end: _,
			own,
			cb,
		} = self;

		let CallBack {
			prologue: _,
			epilogue,
		} = cb;

		epilogue
			.into_iter()
			.zip(own)
			.for_each(|((_, cb), block)| cb(block));
	}

	pub fn perform(&mut self, mut ide: LockedGuard<'_, IdeController>, blocks: Vec<Block>) {
		pr_debug!("+++++ perform called +++++");

		// (write)cache writeback for blocks
		let bmi = unsafe { ide.bmi.assume_init_mut() };
		bmi.set_prd_table(&blocks);
		bmi.set_dma(self.kind);

		self.own = blocks;

		let ata = &mut ide.ata;
		ata.do_dma(self.kind, self.begin, self.count() as u16);

		unsafe { ide.bmi.assume_init_mut().start() };
	}

	pub fn retry(&mut self, ide: LockedGuard<'_, IdeController>) {
		let blocks = take(&mut self.own);
		self.perform(ide, blocks);
	}

	fn count(&self) -> usize {
		self.end - self.begin
	}

	pub fn kilo_bytes(&self) -> usize {
		self.count() * SECTOR_SIZE / KB
	}

	pub fn merge(&mut self, event: Self) {
		debug_assert!(self.kind == event.kind); // ?

		let Self {
			kind: _,
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
