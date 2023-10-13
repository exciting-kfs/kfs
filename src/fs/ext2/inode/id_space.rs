use core::mem::size_of;

use alloc::{collections::VecDeque, sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::ext2::{sb::SuperBlock, Block},
	sync::{LockRW, ReadLockGuard, WriteLockGuard},
	syscall::errno::Errno,
	trace_feature,
};

use super::Inode;

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

	fn nr_id_space(&self, index: usize) -> usize {
		let nr_bid = self.nr_id_in_block();

		index
			.checked_sub(13)
			.map(|n| n / nr_bid)
			.map(|n| if n == 0 { 1 } else { n + 2 })
			.unwrap_or_default()
	}

	fn nr_id_in_block(&self) -> usize {
		self.inode.block_size() / size_of::<u32>()
	}
}

pub struct IdSpaceAdjust<'a> {
	inode: WriteLockGuard<'a, Inode>,
}

impl<'a> IdSpaceAdjust<'a> {
	pub fn new(inode: WriteLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	pub fn adjust(&mut self) -> Result<(), Errno> {
		let sync_len = self.inode.synced_len;
		let data_len = self.inode.chunks.len();

		let old_count = self.nr_id_space(sync_len);
		let new_count = self.nr_id_space(data_len);

		if new_count > old_count {
			self.expand(old_count, new_count)?;
		} else if new_count < old_count {
			self.shrink(old_count, new_count)?;
		}

		self.inode.dirty();
		Ok(())
	}

	fn nr_id_in_block(&self) -> usize {
		self.inode.block_size() / size_of::<u32>()
	}

	fn nr_id_space(&self, index: usize) -> usize {
		let id_count = self.nr_id_in_block();

		index
			.checked_sub(13)
			.map(|n| n / id_count)
			.map(|n| if n == 0 { 1 } else { n + 2 })
			.unwrap_or_default()
	}

	fn shrink(&mut self, old_count: usize, new_count: usize) -> Result<(), Errno> {
		let sb = self.inode.super_block().clone();
		let bids = {
			let mut bids = self.read_id_space(old_count)?;
			bids.truncate(new_count);
			bids
		};

		let mut staged = Vec::new();
		for bid in bids {
			let s = sb.dealloc_block_staged(bid)?;
			staged.push(s);
		}

		if new_count <= 1 {
			self.inode.info.block[14] = 0;
		}
		if new_count == 0 {
			self.inode.info.block[13] = 0;
		}

		staged.iter_mut().for_each(|e| e.commit(()));

		Ok(())
	}

	fn read_id_space(&self, old_count: usize) -> Result<VecDeque<BlockId>, Errno> {
		let mut v = VecDeque::new();

		let sb = self.inode.super_block();
		let info = &self.inode.info;
		let b_13 = info.bid_array(13).unwrap();
		let b_14 = info.bid_array(14).unwrap();

		if b_13.inner() > 0 {
			v.push_front(b_13);
		}

		if b_14.inner() > 0 {
			v.push_front(b_14);

			let id_space = sb.block_pool.get_or_load(b_14)?;
			let id_space = id_space.read_lock();
			let bids = id_space.as_slice_ref_u32();

			(0..(old_count - 2))
				.zip(bids)
				.for_each(|(_, b)| v.push_front(unsafe { BlockId::new_unchecked(*b as usize) }))
		}
		Ok(v)
	}

	fn expand(&mut self, old_count: usize, new_count: usize) -> Result<(), Errno> {
		let sb = self.inode.super_block().clone();
		let block_pool = &sb.block_pool;

		let depth2 = self.inode.info.bid_array(14).unwrap();
		let depth2_need = new_count >= 3;
		let depth2_alloced = depth2.inner() > 0;

		let (id_array, id_space) = self.ready_id_space(depth2, depth2_need, depth2_alloced)?;
		let bids = sb.reserve_blocks(new_count - old_count)?;

		if let Some(id_space) = id_space {
			{
				let mut w_space = id_space.write_lock();
				let space = id_array.iter_mut().chain(w_space.as_slice_mut_u32());
				let space = space.skip(old_count);

				bids.iter().zip(space).for_each(|(s, d)| *d = s.as_u32());
			}

			if !depth2_alloced {
				unsafe {
					let depth2 = self.inode.info.bid_array(14).unwrap();
					block_pool.register(depth2, id_space.clone());
				}
			}
		} else {
			bids.iter()
				.zip(id_array.iter_mut())
				.for_each(|(s, d)| *d = s.as_u32());
		}
		Ok(())
	}

	fn ready_id_space(
		&mut self,
		depth2: BlockId,
		need: bool,
		alloced: bool,
	) -> Result<(&mut [u32], Option<Arc<LockRW<Block>>>), Errno> {
		let sb = self.inode.super_block().clone();
		let block_pool = &sb.block_pool;
		let array = &mut self.inode.info.block;

		Ok(match (need, alloced) {
			(false, _) => (&mut array[13..14], None),
			(true, true) => (&mut array[13..15], Some(block_pool.get_or_load(depth2)?)),
			(true, false) => unsafe {
				(&mut array[13..15], Some(block_pool.unregistered_block()?))
			},
		})
	}
}

pub struct IdSapceWrite<'a> {
	inode: WriteLockGuard<'a, Inode>,
}

impl<'a> IdSapceWrite<'a> {
	pub fn from_adjust(adjust: IdSpaceAdjust<'a>) -> Self {
		let IdSpaceAdjust { inode } = adjust;
		Self { inode }
	}

	pub fn sync_with_data(&mut self) -> Result<(), Errno> {
		let data_len = self.inode.chunks.len();
		let prev_len = self.inode.synced_len;

		if prev_len < data_len {
			let bids = self.blockids();
			let iter = &bids[prev_len..];
			for (index, bid) in (prev_len..data_len).zip(iter) {
				self.write_bid(index, *bid)?;
			}
		} else if prev_len > data_len {
			self.write_bid(data_len, BlockId::zero())?;
		}

		self.inode.synced_len = data_len;
		Ok(())
	}

	fn blockids(&self) -> Vec<BlockId> {
		self.inode.chunks.clone()
	}

	fn nr_id_in_block(&self) -> usize {
		self.inode.block_size() / size_of::<u32>()
	}

	fn load_id_space(&self, id_space_index: usize) -> Result<Arc<LockRW<Block>>, Errno> {
		let block_pool = &self.inode.super_block().block_pool;
		let info = &self.inode.info;

		if id_space_index == 0 {
			let bid = info.bid_array(13).unwrap();
			block_pool.get_or_load(bid)
		} else {
			let bid = info.bid_array(14).unwrap();
			let id_space = block_pool.get_or_load(bid)?;
			let bid = id_space.read_lock().as_slice_ref_u32()[id_space_index - 1];

			if bid > 0 {
				let bid = unsafe { BlockId::new_unchecked(bid as usize) };
				block_pool.get_or_load(bid)
			} else {
				Err(Errno::EINVAL)
			}
		}
	}

	fn write_bid(&mut self, index: usize, bid: BlockId) -> Result<(), Errno> {
		let (id_space_index, local_index) = self.split_index(index);

		if let Some(id_space_index) = id_space_index {
			let block = self.load_id_space(id_space_index)?;
			block.write_lock().as_slice_mut_u32()[local_index] = bid.as_u32();
		} else {
			self.inode.info.block[local_index] = bid.as_u32();
		}

		Ok(())
	}

	fn split_index(&self, index: usize) -> (Option<usize>, usize) {
		let id_count = self.nr_id_in_block();

		match index.checked_sub(13) {
			None => (None, index),
			Some(i) => (Some(i / id_count), i % id_count),
		}
	}
}
