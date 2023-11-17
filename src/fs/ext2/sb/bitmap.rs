use alloc::{sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::ext2::{inode::inum::Inum, Block},
	mm::util::next_align,
	sync::LockRW,
	trace_feature,
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
	nr_blk_in_grp: usize,
	block_size: usize,
}

impl InGroupBid {
	pub fn new(bid: BlockId, info: &SuperBlockInfo) -> Self {
		let count = info.nr_block_in_group() as usize;
		Self {
			bid,
			nr_blk_in_grp: count,
			block_size: info.block_size().as_bytes(),
		}
	}

	pub fn bgid(&self) -> usize {
		self.bid.index(self.block_size) / self.nr_blk_in_grp
	}

	/// Index in block group
	///
	/// ex) nr_blk_in_grp = 8192
	/// - block size: 1024
	///   - block ID: 1 ~ 8192, 8193 ~ 16384, ...
	///   - index: 0 ~ 8191
	///
	/// - block size: 2048
	///   - block ID: 0 ~ 8191, 8192 ~ 16383, ...
	///   - index: 0 ~ 8191
	pub fn index_in_group(&self) -> usize {
		self.bid.index(self.block_size) % self.nr_blk_in_grp
	}

	pub fn count(&self) -> usize {
		self.nr_blk_in_grp
	}
}

pub struct BitMap {
	block: Arc<LockRW<Block>>,
	len: usize,
}

impl BitMap {
	pub fn new(block: &Arc<LockRW<Block>>, len: usize) -> Self {
		BitMap {
			block: block.clone(),
			len,
		}
	}

	pub fn find_free_space(&mut self) -> Option<usize> {
		let mut bitmap = self.block.as_slice_mut_u32();
		let len = next_align(self.len, u32::BITS as usize) / u32::BITS as usize;

		for (i, x) in (0..len).zip(bitmap.iter_mut()) {
			if *x != u32::MAX {
				return Some(i * u32::BITS as usize + x.trailing_ones() as usize);
			}
		}

		None
	}

	pub fn find_free_space_multi(&mut self, count: usize) -> Option<Vec<usize>> {
		let bitmap = self.block.as_slice_mut_u32();
		let len = next_align(self.len, u32::BITS as usize) / u32::BITS as usize;

		let mut v = Vec::new();
		'a: for (i, x) in (0..len).zip(bitmap.iter()) {
			let base = i * u32::BITS as usize;
			let mut x = *x;
			let mut local = x.trailing_ones() as usize;
			while x != u32::MAX {
				if v.len() >= count {
					break 'a;
				}

				let digit = 1 << local;

				if x & digit == 0 {
					v.push(base + local);
					x ^= digit;
				}

				local += 1;
			}
		}

		trace_feature!(
			"ext2-bitmap",
			"bitmap: multi: v_len: {}, count: {}",
			v.len(),
			count
		);

		trace_feature!("ext2-bitmap", "bitmap: indexes: {:?}", v);

		(v.len() == count).then_some(v)
	}

	pub fn toggle_bitmap(&mut self, idx: usize) {
		let idx_h = idx / usize::BITS as usize;
		let idx_l = idx % usize::BITS as usize;

		let mut slice = self.block.as_slice_mut_u32();

		if idx_h < slice.len() {
			slice[idx_h] ^= 1 << idx_l;
		}
	}

	fn into_block(self) -> Arc<LockRW<Block>> {
		self.block.clone()
	}
}
