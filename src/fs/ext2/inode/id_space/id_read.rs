use alloc::{sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::ext2::{inode::Inode, sb::SuperBlock},
	sync::ReadLockGuard,
	syscall::errno::Errno,
	trace_feature,
};

pub struct IdSpaceRead<'a> {
	inode: ReadLockGuard<'a, Inode>,
}

impl<'a> IdSpaceRead<'a> {
	pub fn new(inode: ReadLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	pub fn read_bid(&self) -> Result<Vec<BlockId>, Errno> {
		let sb = &self.inode.super_block();
		let block_info = &self.inode.info.block;
		let mut v = Vec::new();

		if self.__read_bid(sb, &mut v, &block_info[0..12], 0)? {
			return Ok(v);
		}

		if self.__read_bid(sb, &mut v, &block_info[12..13], 1)? {
			return Ok(v);
		}

		if self.__read_bid(sb, &mut v, &block_info[13..14], 2)? {
			return Ok(v);
		}

		if self.__read_bid(sb, &mut v, &block_info[14..15], 3)? {
			return Ok(v);
		}
		Ok(v)
	}

	fn __read_bid(
		&self,
		sb: &Arc<SuperBlock>,
		bid_vec: &mut Vec<BlockId>,
		slice: &[u32],
		depth: usize,
	) -> Result<bool, Errno> {
		if depth == 0 {
			for bid in slice {
				if *bid == 0 {
					return Ok(true);
				}
				let bid = unsafe { BlockId::new_unchecked(*bid as usize) };
				bid_vec.push(bid);
			}
			return Ok(false);
		}

		for bid in slice {
			let bid = unsafe { BlockId::new_unchecked(*bid as usize) };
			trace_feature!(
				"inode-load-bid",
				"read_bid: depth, bid: {}, {:?}",
				depth,
				bid
			);
			let block = sb.block_pool.get_or_load(bid)?;
			let block_read = block.read_lock();
			let slice = block_read.as_slice_ref_u32();

			if self.__read_bid(sb, bid_vec, slice, depth - 1)? {
				return Ok(true);
			}
		}
		Ok(false)
	}
}
