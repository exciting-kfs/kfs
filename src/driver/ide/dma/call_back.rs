use core::alloc::AllocError;

use alloc::{boxed::Box, collections::BTreeMap, vec::Vec};

use crate::driver::ide::{block::Block, lba::LBA28};

pub type Prepare = Box<dyn FnOnce() -> Result<Block, AllocError>>;
pub type Cleanup = Box<dyn FnMut(Result<Block, AllocError>) -> ()>;

pub struct CallBack(BTreeMap<LBA28, (Prepare, Cleanup)>);

impl CallBack {
	pub fn new(start: LBA28, p: Prepare, c: Cleanup) -> Self {
		let mut btree = BTreeMap::new();
		btree.insert(start, (p, c));
		Self(btree)
	}

	pub(super) fn merge(&mut self, mut other: Self) {
		self.0.append(&mut other.0);
	}

	pub(super) fn prepare(self) -> Result<(Vec<Block>, Vec<Cleanup>), Vec<Cleanup>> {
		let callbacks = self.0;

		let mut cleanup = Vec::new();
		let mut blocks = Vec::new();

		for (_, (p, c)) in callbacks {
			cleanup.push(c);
			match p() {
				Ok(block) => blocks.push(block),
				Err(_) => return Err(cleanup),
			}
		}
		Ok((blocks, cleanup))
	}
}
