use core::{alloc::AllocError, mem::take};

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use crate::{
	driver::ide::{block::Block, lba::LBA28, IdeController},
	pr_debug,
	sync::locked::LockedGuard,
};

use super::{read::ReadDma, write::WriteDma, DmaOps};

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

	pub fn merge(&mut self, cb: Self) {
		let Self {
			mut prologue,
			mut epilogue,
		} = cb;
		self.prologue.append(&mut prologue);
		self.epilogue.append(&mut epilogue);
	}
}

pub enum Event {
	Read(ReadDma),
	Write(WriteDma),
}

impl Event {
	pub const MAX_KB: usize = 128;

	pub fn prepare(&mut self) -> Result<Vec<Block>, AllocError> {
		let callbacks = take(&mut self.callback().prologue);
		let results = callbacks.into_iter().map(|(_, cb)| cb());
		let mut blocks = Vec::new();

		for b in results {
			blocks.push(b?);
		}
		Ok(blocks)
	}

	pub fn cleanup(mut self) {
		let own = take(self.own());
		let epilogue = take(&mut self.callback().epilogue);

		epilogue
			.into_iter()
			.zip(own)
			.for_each(|((_, cb), block)| cb(block));
	}

	pub fn perform(&mut self, mut ide: LockedGuard<'_, IdeController>, blocks: Vec<Block>) {
		pr_debug!("+++++ perform called +++++");

		let ops = match self {
			Event::Read(_) => DmaOps::Read,
			Event::Write(_) => DmaOps::Write,
		};

		// (write)cache writeback for blocks
		let bmi = unsafe { ide.bmi.assume_init_mut() };
		bmi.set_prd_table(&blocks);
		bmi.set_dma(ops);

		*self.own() = blocks;

		let ata = &mut ide.ata;
		ata.do_dma(ops, self.begin(), self.count() as u16);

		unsafe { ide.bmi.assume_init_mut().start() };
	}

	pub fn retry(&mut self, ide: LockedGuard<'_, IdeController>) {
		let blocks = take(self.own());
		self.perform(ide, blocks);
	}

	pub fn merge(&mut self, event: Self) {
		match (self, event) {
			(&mut Event::Read(ref mut r1), Event::Read(r2)) => r1.merge(r2),
			(&mut Event::Write(ref mut r1), Event::Write(r2)) => r1.merge(r2),
			_ => {}
		}
	}

	fn callback(&mut self) -> &mut CallBack {
		match self {
			Event::Read(r) => &mut r.cb,
			Event::Write(w) => &mut w.cb,
		}
	}

	fn own(&mut self) -> &mut Vec<Block> {
		match self {
			Event::Read(r) => &mut r.own,
			Event::Write(w) => &mut w.own,
		}
	}

	fn begin(&self) -> LBA28 {
		match self {
			Event::Read(r) => r.begin,
			Event::Write(w) => w.begin,
		}
	}

	pub fn count(&self) -> usize {
		match self {
			Event::Read(r) => r.count(),
			Event::Write(w) => w.count(),
		}
	}
}
