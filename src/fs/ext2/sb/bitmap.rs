use core::mem::size_of;

use alloc::{sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::ext2::{inode::inum::Inum, Block},
	sync::LockRW,
};

use super::info::SuperBlockInfo;

pub struct InGroupInum {
	inum: Inum,
	count: usize,
}

impl InGroupInum {
	pub fn new(inum: Inum, info: &SuperBlockInfo) -> Self {
		let count = info.nr_inode_in_group() as usize;
		Self { inum, count }
	}

	pub fn bgid(&self) -> usize {
		self.inum.index() / self.count
	}

	pub fn index_in_group(&self) -> usize {
		self.inum.index() % self.count
	}

	pub fn count(&self) -> usize {
		self.count
	}
}

pub struct InGroupBid {
	bid: BlockId,
	count: usize,
}

impl InGroupBid {
	pub fn new(bid: BlockId, info: &SuperBlockInfo) -> Self {
		let count = info.nr_block_in_group() as usize;
		Self { bid, count }
	}

	pub fn bgid(&self) -> usize {
		self.bid.inner() / self.count
	}

	pub fn index_in_group(&self) -> usize {
		self.bid.inner() % self.count
	}

	pub fn count(&self) -> usize {
		self.count
	}
}

pub struct BitMap {
	block: Arc<LockRW<Block>>,
	len: usize,
}

impl BitMap {
	pub fn new(block: Arc<LockRW<Block>>, len: usize) -> Self {
		BitMap { block, len }
	}

	pub fn find_free_space(&mut self) -> Option<usize> {
		let mut block = self.block.write_lock();
		let bitmap = block.as_chunks_mut(size_of::<usize>());
		let len = self.len / usize::BITS as usize;

		for (i, mut x) in (0..len).zip(bitmap) {
			let x = unsafe { x.cast::<usize>() };
			if *x != usize::MAX {
				return Some(i * usize::BITS as usize + x.trailing_ones() as usize);
			}
		}

		None
	}

	pub fn find_free_space_multi(&mut self, count: usize) -> Option<Vec<usize>> {
		let mut block = self.block.write_lock();
		let bitmap = block.as_chunks_mut(size_of::<usize>());
		let len = self.len / usize::BITS as usize;

		let mut v = Vec::new();
		for (i, mut x) in (0..len).zip(bitmap) {
			let base = i * usize::BITS as usize;
			let mut x = *unsafe { x.cast::<usize>() };
			let mut local = x.trailing_ones() as usize;
			while x != usize::MAX {
				if v.len() >= count {
					break;
				}

				let digit = 1 << local;

				if x & digit == 0 {
					v.push(base + local);
					x ^= digit;
					local += 1;
				} else {
					local = x.trailing_ones() as usize;
				}
			}
		}

		// pr_debug!("bitmap: multi: count: {}, v_len: {}", count, v.len());

		(v.len() == count).then_some(v)
	}

	pub fn toggle_bitmap(&mut self, idx: usize) {
		let idx_h = idx / usize::BITS as usize;
		let idx_l = idx % usize::BITS as usize;

		let mut block = self.block.write_lock();
		let mut bitmap = block.as_chunks_mut(size_of::<usize>()).skip(idx_h);
		let chunk = bitmap.next();

		if let Some(mut x) = chunk {
			let x = unsafe { x.cast::<usize>() };
			*x ^= 1 << idx_l;
		}
	}

	fn into_block(self) -> Arc<LockRW<Block>> {
		self.block.clone()
	}
}
