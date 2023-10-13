use core::{cmp::min, mem::size_of};

use alloc::{
	sync::Arc,
	vec::{self, Vec},
};

use crate::{
	driver::partition::BlockId, fs::ext2::block_pool::BlockPool, sync::WriteLockGuard,
	syscall::errno::Errno, trace_feature,
};

use super::{Chunk, Command, Depth, IdSpaceAdjust, IdxInBlk, Inode, StackHelper};

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
			let mut bids = self.blockids();
			let end = min(bids.len(), 12);

			bids.push(unsafe { BlockId::new_unchecked(0) });

			let mut stream = bids.into_iter();
			for _ in 0..prev_len {
				stream.next();
			}

			for i in prev_len..end {
				if let Some(bid) = stream.next() {
					self.inode.info.block[i] = bid.as_u32();
				}
			}

			let stack = self.prepare_stack(prev_len)?;
			let pool = self.inode.block_pool();

			trace_feature!(
				"ext2-idspace",
				"sync_wit_data: stack: {:?}, stream_len {}",
				stack,
				stream.len()
			);

			IdWriter::new(pool, stream, stack).write()?;
		} else if prev_len > data_len {
			for i in data_len..12 {
				self.inode.info.block[i] = 0;
			}

			let mut stack = self.prepare_stack(data_len)?;
			if let Some(mut depth) = stack.pop() {
				depth.chunk().slice()[0] = 0;
			}
		}

		trace_feature!(
			"ext2-idspace",
			"sync_wit_data: array: {:?}",
			self.inode.info.block
		);

		self.inode.synced_len = data_len;
		Ok(())
	}

	fn blockids(&self) -> Vec<BlockId> {
		self.inode
			.chunks
			.iter()
			.map(|b| *b.lock().block_id())
			.collect::<Vec<_>>()
	}

	#[inline]
	fn nr_id_in_block(&self) -> usize {
		self.inode.block_size() / size_of::<u32>()
	}

	fn stack_helper(&self) -> StackHelper<'_> {
		StackHelper::new(&self.inode)
	}

	fn prepare_stack(&self, index: usize) -> Result<Vec<Depth>, Errno> {
		let id_count = self.nr_id_in_block();

		let indexes = IdxInBlk::split(index, id_count);
		let mut helper = self.stack_helper();

		match indexes {
			IdxInBlk::Depth3 { mut blk_i } => {
				blk_i[2] -= 1;

				helper.push_block_slice(14, &blk_i)?;
				Ok(helper.into_stack())
			}
			IdxInBlk::Depth2 { mut blk_i } => {
				blk_i[1] -= 1;

				helper.push_block_one(14, 2)?;
				helper.push_block_slice(13, &blk_i)?;

				Ok(helper.into_stack())
			}
			IdxInBlk::Depth1 { mut blk_i } => {
				blk_i[0] -= 1;

				helper.push_block_one(14, 2)?;
				helper.push_block_one(13, 1)?;
				helper.push_block_slice(12, &blk_i)?;

				Ok(helper.into_stack())
			}
			IdxInBlk::Depth0 => {
				helper.push_block_one(14, 2)?;
				helper.push_block_one(13, 1)?;
				helper.push_block_one(12, 0)?;

				Ok(helper.into_stack())
			}
		}
	}
}

struct IdWriter {
	block_pool: Arc<BlockPool>,
	stack: Vec<Depth>,
	stream: vec::IntoIter<BlockId>,
}

impl IdWriter {
	fn new(block_pool: &Arc<BlockPool>, stream: vec::IntoIter<BlockId>, stack: Vec<Depth>) -> Self {
		Self {
			block_pool: block_pool.clone(),
			stack,
			stream,
		}
	}

	pub fn write(&mut self) -> Result<(), Errno> {
		let stack = &mut self.stack;
		let stream = &mut self.stream;

		while !stream.is_empty() {
			// pr_debug!("write: stack: {:?}", stack);
			let top = match stack.last_mut() {
				None => break,
				Some(top) => top,
			};

			let command = Self::__run(&self.block_pool, top, stream)?;

			match command {
				Command::End => break,
				Command::Push(lv) => stack.push(lv),
				Command::Pop => {
					stack.pop();
				}
			}
		}
		Ok(())
	}

	fn __run(
		block_pool: &Arc<BlockPool>,
		top: &mut Depth,
		stream: &mut vec::IntoIter<BlockId>,
	) -> Result<Command, Errno> {
		match top {
			Depth::Zero(chunk) => {
				for s in chunk.slice().iter_mut() {
					let bid = match stream.next() {
						Some(id) => id,
						None => return Ok(Command::End),
					};
					*s = bid.as_u32();
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

				let len = block.read_lock().size() / size_of::<u32>();
				let chunk = Chunk::new(&block, 0..len);
				let level = Depth::new(*level - 1, chunk);

				Ok(Command::Push(level))
			}
		}
	}
}
