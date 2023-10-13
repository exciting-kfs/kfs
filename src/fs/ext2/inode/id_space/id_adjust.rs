use core::mem::size_of;

use alloc::{
	sync::Arc,
	vec::{self, Vec},
};

use super::{Chunk, Command, Depth, StackHelper};
use crate::{
	driver::partition::BlockId,
	fs::ext2::{block_pool::BlockPool, inode::Inode, Block},
	sync::{LockRW, WriteLockGuard},
	syscall::errno::Errno,
	trace_feature,
};

use super::Indexes;

pub struct IdSpaceAdjust<'a> {
	pub inode: WriteLockGuard<'a, Inode>,
}

impl<'a> IdSpaceAdjust<'a> {
	pub fn new(inode: WriteLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	pub fn adjust(&mut self) -> Result<(), Errno> {
		let sync_len = self.inode.synced_len + 1;
		let data_len = self.inode.chunks.len() + 1;

		let old_count = self.nr_id_space(sync_len);
		let new_count = self.nr_id_space(data_len);

		trace_feature!(
			"ext2-idspace",
			"adjust: old_count: {}, new_count: {}",
			old_count,
			new_count
		);

		if new_count > old_count {
			self.expand(new_count - old_count)?;
		} else if new_count < old_count {
			self.shrink(old_count, new_count)?;
		}

		self.inode.dirty();
		Ok(())
	}

	#[inline]
	fn nr_id_in_block(&self) -> usize {
		self.inode.block_size() / size_of::<u32>()
	}

	fn nr_id_space(&self, index: usize) -> usize {
		let c = self.nr_id_in_block();
		let mut sum = 0;

		sum += index.checked_sub(12).map(|_| 1).unwrap_or_default();
		sum += index
			.checked_sub(c + 12)
			.map(|x| x / c + 2)
			.unwrap_or_default();

		sum += index
			.checked_sub(c * c + c + 12)
			.map(|x| x / (c * c) + x / c + 3)
			.unwrap_or_default();

		sum
	}

	fn shrink(&mut self, old_count: usize, new_count: usize) -> Result<(), Errno> {
		let sb = self.inode.super_block();

		let stack = self.prepare_stack_shrink()?;
		let pool = self.inode.block_pool();
		let bids = SpaceReader::new(pool, stack, old_count - new_count).read()?;

		let mut staged = Vec::new();
		for bid in bids {
			let s = sb.dealloc_block_staged(bid)?;
			staged.push(s);
		}

		if new_count <= self.nr_id_in_block() + 2 {
			self.inode.info.block[14] = 0;
		}
		if new_count <= 1 {
			self.inode.info.block[13] = 0;
		}
		if new_count == 0 {
			self.inode.info.block[12] = 0;
		}

		staged.iter_mut().for_each(|e| e.commit(()));

		Ok(())
	}

	fn expand(&mut self, count: usize) -> Result<(), Errno> {
		let sb = self.inode.super_block();

		let mut stack = self.prepare_stack_expand()?;

		let blocks = sb.alloc_blocks(count)?;
		let mut blocks = blocks.into_iter();

		if let Some(mut v) = self.alloc_idspace(&mut blocks) {
			v.append(&mut stack);
			stack = v;
		}

		trace_feature!("ext2-idspace", "expand: stack: {:?}", stack);

		let mut expander = SpaceExpander::new(blocks, stack);
		expander.expand();

		Ok(())
	}

	fn stack_helper(&self) -> StackHelper<'_> {
		StackHelper::new(&self.inode)
	}

	fn prepare_stack_shrink(&self) -> Result<Vec<Depth>, Errno> {
		let mut helper = self.stack_helper();
		let id_count = self.nr_id_in_block();
		let indexes = Indexes::split(self.inode.chunks.len(), id_count);

		match indexes {
			Indexes::Depth3 { blk_i } => {
				helper.push_block_slice_recursive(14, &blk_i[..2])?;

				Ok(helper.into_stack())
			}
			Indexes::Depth2 { blk_i } => {
				helper.push_block_one(14, 1)?;
				helper.push_block_slice_recursive(13, &blk_i[..1])?;

				Ok(helper.into_stack())
			}
			_ => Ok(Vec::new()),
		}
	}

	fn prepare_stack_expand(&self) -> Result<Vec<Depth>, Errno> {
		let id_count = self.nr_id_in_block();
		let indexes = Indexes::split(self.inode.synced_len, id_count);
		let mut helper = self.stack_helper();

		match indexes {
			Indexes::Depth3 { blk_i } => {
				helper.push_block_slice_recursive(14, &blk_i[..2])?;
				Ok(helper.into_stack())
			}
			Indexes::Depth2 { blk_i } => {
				helper.push_block_slice_recursive(13, &blk_i[..1])?;
				Ok(helper.into_stack())
			}
			_ => Ok(Vec::new()),
		}
	}

	fn alloc_idspace(
		&mut self,
		iter: &mut vec::IntoIter<Arc<LockRW<Block>>>,
	) -> Option<Vec<Depth>> {
		let id_count = self.nr_id_in_block();
		let indexes = Indexes::split(self.inode.chunks.len(), id_count);
		let array = &mut self.inode.info.block;
		let last = indexes.array_index();

		let mut v = Vec::new();

		if last >= 12 && array[12] == 0 {
			let block = iter.next()?;
			array[12] = block.read_lock().id().as_u32();
		}

		for i in 13..=last {
			if array[i] == 0 {
				let block = iter.next()?;
				array[i] = block.read_lock().id().as_u32();
				let chunk = Chunk::new(&block, 0..id_count);
				let depth = Depth::new(i - 13, chunk);
				v.push(depth);
			}
		}

		trace_feature!("ext2-idspace", "alloc_idspace: array: {:?}", array);

		v.reverse();

		Some(v)
	}
}

pub struct SpaceExpander {
	stack: Vec<Depth>,
	stream: vec::IntoIter<Arc<LockRW<Block>>>,
}

impl SpaceExpander {
	pub fn new(blocks: vec::IntoIter<Arc<LockRW<Block>>>, stack: Vec<Depth>) -> Self {
		Self {
			stack,
			stream: blocks,
		}
	}

	pub fn expand(&mut self) -> Option<()> {
		let stack = &mut self.stack;
		let stream = &mut self.stream;

		loop {
			let top = stack.last_mut()?;
			let command = Self::__run(top, stream)?;

			match command {
				Command::End => break Some(()),
				Command::Push(lv) => stack.push(lv),
				Command::Pop => {
					stack.pop();
				}
			}
		}
	}

	fn __run(top: &mut Depth, stream: &mut vec::IntoIter<Arc<LockRW<Block>>>) -> Option<Command> {
		match top {
			Depth::Zero(chunk) => {
				for s in chunk.slice().iter_mut() {
					let block = stream.next()?;
					let bid = block.read_lock().id();
					*s = bid.as_u32();
				}
				Some(Command::Pop)
			}
			Depth::NonZero(level, chunk) => {
				let mut s = match chunk.split_first() {
					Some(s) => s,
					None => return Some(Command::Pop),
				};

				let block = stream.next().expect("always");
				let bid = block.read_lock().id();
				s[0] = bid.as_u32();

				let len = block.read_lock().size() / size_of::<u32>();
				let chunk = Chunk::new(&block, 0..len);
				let level = Depth::new(*level - 1, chunk);
				Some(Command::Push(level))
			}
		}
	}
}

struct SpaceReader {
	block_pool: Arc<BlockPool>,
	stack: Vec<Depth>,
	count: usize,
}

impl SpaceReader {
	pub fn new(block_pool: &Arc<BlockPool>, stack: Vec<Depth>, count: usize) -> Self {
		Self {
			block_pool: block_pool.clone(),
			stack,
			count,
		}
	}

	pub fn read(&mut self) -> Result<Vec<BlockId>, Errno> {
		let stack = &mut self.stack;
		let mut v = Vec::new();

		loop {
			let top = match stack.last_mut() {
				None => break Ok(v),
				Some(top) => top,
			};

			let command = Self::__run(&self.block_pool, top, &mut v, self.count)?;

			match command {
				Command::End => break Ok(v),
				Command::Push(lv) => stack.push(lv),
				Command::Pop => {
					stack.pop();
				}
			}
		}
	}

	fn __run(
		block_pool: &Arc<BlockPool>,
		top: &mut Depth,
		basket: &mut Vec<BlockId>,
		end: usize,
	) -> Result<Command, Errno> {
		match top {
			Depth::Zero(chunk) => {
				for s in chunk.slice().iter_mut() {
					basket.push(unsafe { BlockId::new_unchecked(*s as usize) });
					if basket.len() >= end {
						return Ok(Command::End);
					}
				}
				Ok(Command::Pop)
			}
			Depth::NonZero(level, chunk) => {
				let s = match chunk.split_first() {
					Some(s) => s,
					None => return Ok(Command::Pop),
				};

				let bid = unsafe { BlockId::new_unchecked(s[0] as usize) };
				let block = block_pool.get_or_load(bid)?;
				basket.push(bid);
				debug_assert!(basket.len() < end);

				let len = block.read_lock().size() / size_of::<u32>();
				let chunk = Chunk::new(&block, 0..len);
				let level = Depth::new(*level - 1, chunk);

				Ok(Command::Push(level))
			}
		}
	}
}
