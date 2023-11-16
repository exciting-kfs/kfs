use core::{
	cmp::min,
	fmt::{Debug, Display},
	mem::{replace, size_of, take},
};

use alloc::{boxed::Box, vec::Vec};

use crate::{
	driver::partition::BlockId, fs::ext2::inode::inum::Inum, sync::LocalLocked, write_field,
};

use super::info::SuperBlockInfo;

/// Block Group Descripotr
#[repr(C)]
#[derive(PartialEq)]
pub struct BGD {
	block_bitmap: u32,
	inode_bitmap: u32,
	inode_table: u32,
	pub free_blocks_count: u16,
	pub free_inodes_count: u16,
	pub used_dirs_count: u16,
	_pad: u16,
	_reserved: [u32; 3],
}

impl BGD {
	#[inline]
	pub fn inode_bitmap(&self) -> BlockId {
		unsafe { BlockId::new_unchecked(self.inode_bitmap as usize) }
	}

	#[inline]
	pub fn block_bitmap(&self) -> BlockId {
		unsafe { BlockId::new_unchecked(self.block_bitmap as usize) }
	}

	pub fn block_of_inode(&self, inum: Inum, info: &SuperBlockInfo) -> BlockId {
		let block_offset = info.inode_index_in_group(inum) / info.nr_inode_in_block();
		let bid = self.inode_table as usize + block_offset;

		unsafe { BlockId::new_unchecked(bid) }
	}
}

impl Debug for BGD {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		Display::fmt(&self, f)
	}
}

impl Display for BGD {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "\n")?;
		write_field!(f, self, block_bitmap)?;
		write_field!(f, self, inode_bitmap)?;
		write_field!(f, self, inode_table)?;
		write_field!(f, self, free_blocks_count)?;
		write_field!(f, self, free_inodes_count)?;
		write_field!(f, self, used_dirs_count)?;

		Ok(())
	}
}

impl BGD {
	fn test_new(i: usize) -> Self {
		Self {
			block_bitmap: i as u32,
			inode_bitmap: i as u32,
			inode_table: i as u32,
			free_blocks_count: i as u16,
			free_inodes_count: i as u16,
			used_dirs_count: i as u16,
			_pad: i as u16,
			_reserved: [0; 3],
		}
	}
}

pub struct FreeBGD<'a> {
	bgd: &'a mut BGD,
	gid: usize,
	count: usize,
}

impl<'a> FreeBGD<'a> {
	fn new(bgd: &'a mut BGD, gid: usize, count: usize) -> Self {
		Self { bgd, gid, count }
	}

	#[inline]
	pub fn block_bitmap(&self) -> BlockId {
		self.bgd.block_bitmap()
	}

	#[inline]
	pub fn gid(&self) -> usize {
		self.gid
	}

	#[inline]
	pub fn free_count(&self) -> usize {
		self.count
	}

	#[inline]
	pub fn dec_free_blocks_count(&mut self, count: usize) {
		self.bgd.free_blocks_count -= count as u16;
	}
}

pub struct BGDT(Vec<Box<[BGD]>>);

impl BGDT {
	pub fn new(v: Vec<Box<[BGD]>>) -> Option<Self> {
		if v.len() == 0 {
			None
		} else {
			Some(Self(v))
		}
	}

	pub fn bgd_of_bid(&self, bid: BlockId, info: &SuperBlockInfo) -> &BGD {
		let bgid = info.bid_to_bgid(bid);
		self.get_bgd(bgid).unwrap()
	}

	pub fn bgd_of_bid_mut(&mut self, bid: BlockId, info: &SuperBlockInfo) -> &mut BGD {
		let bgid = info.bid_to_bgid(bid);
		self.get_bgd_mut(bgid).unwrap()
	}

	pub fn bgd_of_inum(&self, inum: Inum, info: &SuperBlockInfo) -> &BGD {
		let bgid = info.inum_to_bgid(inum);
		self.get_bgd(bgid).unwrap()
	}

	pub fn bgd_of_inum_mut(&mut self, inum: Inum, info: &SuperBlockInfo) -> &mut BGD {
		let bgid = info.inum_to_bgid(inum);
		self.get_bgd_mut(bgid).unwrap()
	}

	pub fn get_bgd_mut(&mut self, bgid: usize) -> Option<&mut BGD> {
		self.bgid_to_index(bgid).map(|(x, y)| &mut self.0[x][y])
	}

	fn get_bgd(&self, bgid: usize) -> Option<&BGD> {
		self.bgid_to_index(bgid).map(|(x, y)| &self.0[x][y])
	}

	fn bgid_to_index(&self, bgid: usize) -> Option<(usize, usize)> {
		let chunk_index = bgid / self.nr_bgd_in_chunk();

		if chunk_index >= self.nr_chunk() {
			return None;
		}

		let local_index = bgid % self.nr_bgd_in_chunk();
		Some((chunk_index, local_index))
	}

	pub fn find_groups(&mut self, mut count: usize) -> Option<Vec<FreeBGD>> {
		let mut v = Vec::new();
		let mut bgid = 0;

		'a: for chunk in self.0.iter_mut() {
			for bgd in chunk.iter_mut() {
				let free = bgd.free_blocks_count as usize;
				match free.checked_sub(count) {
					Some(_) => {
						v.push(FreeBGD::new(bgd, bgid, count));
						count = 0;
						break 'a;
					}
					None => {
						v.push(FreeBGD::new(bgd, bgid, free));
						count -= free;
					}
				}
				bgid += 1;
			}
		}
		(count == 0).then_some(v)
	}

	pub fn find_bgd<F>(&mut self, condition: F) -> Option<(usize, &mut BGD)>
	where
		F: Fn(&mut BGD) -> bool,
	{
		let mut bgid = 0;
		for chunk in self.0.iter_mut() {
			for bgd in chunk.iter_mut() {
				if condition(bgd) {
					return Some((bgid, bgd));
				}
				bgid += 1;
			}
		}
		None
	}

	#[inline]
	fn nr_chunk(&self) -> usize {
		self.0.len()
	}

	#[inline]
	fn nr_bgd_in_chunk(&self) -> usize {
		self.0[0].len()
	}
}

impl LocalLocked<BGDT> {
	pub fn iter(&self, block_size: usize) -> Iter<'_> {
		Iter::new(self, block_size)
	}
}

pub struct Iter<'a> {
	table: &'a LocalLocked<BGDT>,
	chunks: &'a [Box<[BGD]>],
	chunk: &'a [BGD],
	nr_bgd: usize,
	index: usize,
}

impl<'a> Iter<'a> {
	fn new(table: &'a LocalLocked<BGDT>, block_size: usize) -> Self {
		let nr_bgd = block_size / size_of::<BGD>();

		let chunks = unsafe { table.lock_manual().0.as_slice() };
		let (chunk, chunks) = chunks.split_at(1);
		let chunk = chunk[0].as_ref();

		Self {
			table,
			chunks,
			chunk,
			nr_bgd,
			index: 0,
		}
	}
}

impl<'a> Drop for Iter<'a> {
	fn drop(&mut self) {
		unsafe { self.table.unlock_manual() }
	}
}

impl<'a> Iterator for Iter<'a> {
	type Item = &'a [BGD];
	fn next(&mut self) -> Option<Self::Item> {
		if self.chunk.len() == 0 {
			return None;
		}

		let chunk = take(&mut self.chunk);
		let at = min(self.nr_bgd, chunk.len());

		let (slice, remain) = chunk.split_at(at);

		if remain.len() != 0 {
			let _ = replace(&mut self.chunk, remain);
		} else {
			let chunks = take(&mut self.chunks);
			if let Some((one, remain)) = chunks.split_first() {
				let _ = replace(&mut self.chunk, one.as_ref());
				let _ = replace(&mut self.chunks, remain);
			}
		}

		Some(slice)
	}
}

mod test {
	use core::mem::size_of;

	use alloc::{boxed::Box, vec::Vec};
	use kfs_macro::ktest;

	use crate::{interrupt::leave_interrupt_context, sync::LocalLocked};

	use super::{BGD, BGDT};
	#[ktest(bgd)]
	fn iter_test() {
		let block_size = 64;
		let chunk_size = 128;
		let nr_chunk = 3;

		let mut v = Vec::new();

		for i in 0..nr_chunk {
			let mut chunk = Box::<[BGD]>::new_uninit_slice(chunk_size);
			let b = unsafe {
				for j in 0..chunk_size {
					chunk[j]
						.as_mut_ptr()
						.write(BGD::test_new(i * chunk_size + j));
				}
				chunk.assume_init()
			};

			v.push(b);
		}

		let table = BGDT::new(v).unwrap();
		let table = LocalLocked::new(table);

		unsafe { leave_interrupt_context() };
		let iter = table.iter(block_size);
		let nr_bgd = block_size / size_of::<BGD>();

		for (i, bgd) in iter.enumerate() {
			for j in 0..nr_bgd {
				assert_eq!(BGD::test_new(2 * i + j), bgd[j]);
			}
		}
	}
}
