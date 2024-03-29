use core::alloc::AllocError;

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::driver::ide::{block::Block, lba::LBA28};

pub type Prepare = Box<dyn FnOnce() -> Result<Block, AllocError>>;
pub type Cleanup = Box<dyn FnMut(Result<Block, AllocError>) -> ()>;

pub struct OwnHook(BTreeMap<LBA28, (Prepare, Cleanup)>);

impl OwnHook {
	pub fn new(start: LBA28, p: Prepare, c: Cleanup) -> Self {
		let mut btree = BTreeMap::new();
		btree.insert(start, (p, c));
		Self(btree)
	}

	pub(super) fn merge(&mut self, other: &mut Self) {
		self.0.append(&mut other.0);
	}

	pub(super) fn prepare(self) -> Result<(Vec<Block>, Vec<Cleanup>), Vec<Cleanup>> {
		let hooks = self.0;
		let len = hooks.len();

		let mut cleanup = Vec::new();
		let mut blocks = Vec::new();

		cleanup.reserve(len);
		blocks.reserve(len);

		for (_, (p, c)) in hooks {
			cleanup.push(c);
			match p() {
				Ok(block) => blocks.push(block),
				Err(_) => return Err(cleanup),
			}
		}
		Ok((blocks, cleanup))
	}
}

pub type ItemWB = Arc<dyn WriteBack>;
pub type PrepareWB = Box<dyn FnOnce() -> ItemWB>;
pub type CleanupWB = Box<dyn FnMut(ItemWB) -> ()>;

pub struct WBHook(BTreeMap<LBA28, (PrepareWB, CleanupWB)>);

impl WBHook {
	pub fn new(start: LBA28, p: PrepareWB, c: CleanupWB) -> Self {
		let mut btree = BTreeMap::new();
		btree.insert(start, (p, c));
		Self(btree)
	}

	pub(super) fn merge(&mut self, other: &mut Self) {
		self.0.append(&mut other.0);
	}

	pub(super) fn prepare(self) -> (Vec<ItemWB>, Vec<CleanupWB>) {
		let callbacks = self.0;

		let mut cleanup = Vec::new();
		let mut blocks = Vec::new();

		for (_, (p, c)) in callbacks {
			cleanup.push(c);
			blocks.push(p())
		}

		(blocks, cleanup)
	}
}

pub trait WriteBack {
	fn as_phys_addr(&self) -> usize;
	fn size(&self) -> usize;
	fn prepare(&self) {}
	fn cleanup(&self) {}
}
