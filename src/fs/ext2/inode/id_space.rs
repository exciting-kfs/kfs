mod id_adjust;
mod id_read;
mod id_write;

use super::Inode;

pub use self::id_adjust::IdSpaceAdjust;
pub use self::id_read::IdSpaceRead;
pub use self::id_write::IdSapceWrite;

use core::{mem::size_of, ops::Range};

use alloc::{sync::Arc, vec::Vec};

use crate::{
	driver::partition::BlockId,
	fs::ext2::{
		block_pool::block::{Slice32, SliceMut32},
		Block,
	},
	sync::LockRW,
	syscall::errno::Errno,
};

enum Command {
	Push(Depth),
	Pop,
	End,
}

enum IdxInBlk {
	Depth0,
	Depth1 { blk_i: [isize; 1] },
	Depth2 { blk_i: [isize; 2] },
	Depth3 { blk_i: [isize; 3] },
}

impl IdxInBlk {
	fn split(index: usize, id_count: usize) -> Self {
		let c = id_count;
		if index >= c * c + c + 12 {
			let d1 = (index - 12 - c) / (c * c) - 1;
			let d2 = ((index - 12) / c - 1) % c;
			let d3 = (index - 12) % c;
			IdxInBlk::Depth3 {
				blk_i: [d1 as isize, d2 as isize, d3 as isize],
			}
		} else if index >= c + 12 {
			let d1 = (index - 12) / c - 1;
			let d2 = (index - 12) % c;

			IdxInBlk::Depth2 {
				blk_i: [d1 as isize, d2 as isize],
			}
		} else if index >= 12 {
			IdxInBlk::Depth1 {
				blk_i: [index as isize - 12],
			}
		} else {
			IdxInBlk::Depth0
		}
	}

	fn array_index(&self) -> usize {
		match self {
			Self::Depth0 => 0,
			Self::Depth1 { blk_i: _ } => 12,
			Self::Depth2 { blk_i: _ } => 13,
			Self::Depth3 { blk_i: _ } => 14,
		}
	}
}

pub enum Depth {
	Zero(Chunk),
	NonZero(usize, Chunk),
}

impl Depth {
	pub fn new(depth: usize, chunk: Chunk) -> Self {
		match depth {
			0 => Self::Zero(chunk),
			x => Self::NonZero(x, chunk),
		}
	}

	fn chunk(&mut self) -> &mut Chunk {
		match self {
			Self::NonZero(_, chunk) => chunk,
			Self::Zero(chunk) => chunk,
		}
	}
}

impl core::fmt::Debug for Depth {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Zero(c) => write!(f, "lv: 0, rng: {:?}", c.range()),
			Self::NonZero(lv, c) => write!(f, "lv: {}, rng: {:?}", lv, c.range()),
		}
	}
}

#[derive(Debug)]
pub struct Chunk {
	block: Arc<LockRW<Block>>,
	range: Range<usize>,
}

impl Chunk {
	pub fn new(block: &Arc<LockRW<Block>>, range: Range<usize>) -> Self {
		Self {
			block: block.clone(),
			range,
		}
	}

	fn slice(&mut self) -> Slice32<'_> {
		Slice32::new(&mut self.block, self.range.clone())
	}

	fn slice_mut(&mut self) -> SliceMut32<'_> {
		SliceMut32::new(&mut self.block, self.range.clone())
	}

	fn len(&self) -> usize {
		self.range.len()
	}

	fn split_first(&mut self) -> Option<SliceMut32<'_>> {
		if self.len() == 0 {
			return None;
		}

		let block = &mut self.block;
		let rng = &mut self.range;
		let start = rng.start;

		*rng = rng.start + 1..rng.end;
		Some(SliceMut32::new(block, start..start + 1))
	}

	fn range(&self) -> &Range<usize> {
		&self.range
	}
}

struct StackHelper<'a> {
	inode: &'a Inode,
	stack: Vec<Depth>,
	id_count: usize,
}

impl<'a> StackHelper<'a> {
	fn new(inode: &'a Inode) -> Self {
		let id_count = inode.block_size() / size_of::<u32>();
		Self {
			inode,
			stack: Vec::new(),
			id_count,
		}
	}

	fn into_stack(self) -> Vec<Depth> {
		let Self {
			inode: _,
			stack,
			id_count: _,
		} = self;

		stack
	}

	fn push_block_one(&mut self, arr_i: usize, depth: usize) -> Result<(), Errno> {
		let block_pool = self.inode.block_pool();

		if self.inode.info.block[arr_i] > 0 {
			let bid = self.inode.info.bid_array(arr_i).unwrap();
			let block = block_pool.get_or_load(bid)?;

			let chunk = Chunk::new(&block, 0..self.id_count);
			let depth = Depth::new(depth, chunk);

			self.stack.push(depth);
		}

		Ok(())
	}

	fn push_block_slice(&mut self, arr_i: usize, blk_i: &[isize]) -> Result<(), Errno> {
		let block_pool = self.inode.block_pool();
		let bid = self.inode.info.bid_array(arr_i).unwrap();
		let idspace = block_pool.get_or_load(bid)?;
		let depth = blk_i.len() - 1;

		let start = (blk_i[0] + 1) as usize;
		let chunk = Chunk::new(&idspace, start..self.id_count);
		self.stack.push(Depth::new(depth, chunk));

		self.__push_block_slice(&idspace, &blk_i, depth)?;

		Ok(())
	}

	fn __push_block_slice(
		&mut self,
		idspace: &Arc<LockRW<Block>>,
		blk_i: &[isize],
		depth: usize,
	) -> Result<(), Errno> {
		let block_pool = self.inode.block_pool();

		for dep in (1..=depth).rev() {
			let (first, blk_i) = blk_i.split_first().expect("check slice length");
			let bid = idspace.as_slice_ref_u32()[*first as usize];
			let bid = unsafe { BlockId::new_unchecked(bid as usize) };

			let block = block_pool.get_or_load(bid)?;

			let start = (blk_i[0] + 1) as usize;
			let chunk = Chunk::new(&block, start..self.id_count);
			let depth = Depth::new(dep - 1, chunk);

			self.stack.push(depth);
		}

		Ok(())
	}
}
