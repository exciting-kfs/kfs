use core::{
	fmt::{Debug, Display},
	mem::size_of,
};

use crate::{
	driver::{hpet::get_timestamp_second, ide::block::BlockSize, partition::BlockId},
	fs::ext2::inode::inum::Inum,
	process::task::CURRENT,
	write_field,
};

use super::bgd::BGD;

#[repr(u16)]
enum FsState {
	Valid = 1,
	Error = 2,
}

#[derive(Clone)]
#[repr(C)]
pub struct SuperBlockInfo {
	inodes_count: u32,
	blocks_count: u32,
	r_blocks_count: u32,
	free_blocks_count: u32,
	free_inodes_count: u32,
	first_data_block: u32,
	log_block_size: u32,
	log_frag_size: u32,
	blocks_per_group: u32,
	frags_per_group: u32,
	inodes_per_group: u32,
	mtime: u32,
	pub(super) wtime: u32,
	mnt_count: u16,
	max_mnt_count: u16,
	magic: u16,
	state: u16,
	errors: u16, // ?
	minor_rev_level: u16,
	lastcheck: u32,     // ?
	checkinterval: u32, // ?
	creator_os: u32,
	rev_level: u32,
	def_resuid: u16,
	def_resgid: u16,
	first_ino: u32,
	inode_size: u16,
	block_group_nr: u16,
	feature_compat: u32,
	feature_incompat: u32,
	feature_ro_compat: u32,
	uuid: [u8; 16],
	volume_name: [u8; 16],
	last_mounted: [u8; 64],
	algo_bitmap: u32,
	prealloc_blocks: u8,
	prealloc_dir_blocks: u8,
	_pad: u16,
}

impl SuperBlockInfo {
	#[inline]
	pub fn inode_size(&self) -> usize {
		self.inode_size as usize
	}

	#[inline]
	pub fn nr_inode_in_group(&self) -> usize {
		self.inodes_per_group as usize
	}

	#[inline]
	pub fn nr_block_in_group(&self) -> usize {
		self.blocks_per_group as usize
	}

	#[inline]
	pub fn nr_group(&self) -> usize {
		((self.blocks_count - 1) / self.blocks_per_group + 1) as usize
	}

	#[inline]
	pub fn bgdt_size(&self) -> usize {
		// Max: 8MB
		self.nr_group() * size_of::<BGD>()
	}

	#[inline]
	pub fn block_size(&self) -> BlockSize {
		BlockSize::from_bytes(1024 << self.log_block_size).unwrap()
	}

	#[inline]
	pub fn nr_inode_in_block(&self) -> usize {
		self.block_size().as_bytes() / self.inode_size as usize
	}

	#[inline]
	pub fn inum_to_bgid(&self, inum: Inum) -> usize {
		inum.index() / self.inodes_per_group as usize
	}

	#[inline]
	pub fn bid_to_bgid(&self, bid: BlockId) -> usize {
		bid.inner() / self.blocks_per_group as usize
	}

	#[inline]
	pub fn inode_index_in_group(&self, inum: Inum) -> usize {
		inum.index() % self.inodes_per_group as usize
	}

	#[inline]
	pub fn inode_index_in_block(&self, inum: Inum) -> usize {
		self.inode_index_in_group(inum) % self.nr_inode_in_block()
	}

	pub fn bgdt_bid(&self) -> BlockId {
		let base = 1024 / self.block_size().as_bytes();

		unsafe { BlockId::new_unchecked(base + 1) }
	}

	pub fn bitmap_index_to_inum(&self, bgid: usize, index: usize) -> Inum {
		let num = bgid * self.inodes_per_group as usize + index + 1;
		unsafe { Inum::new_unchecked(num) }
	}

	pub fn bitmap_index_to_block_id(&self, bgid: usize, index: usize) -> BlockId {
		unsafe {
			BlockId::new_unchecked(
				bgid * self.blocks_per_group as usize
					+ index + (1024 / self.block_size().as_bytes()),
			)
		}
	}

	pub fn sb_backup_bid(&self, buf: &mut [usize; 5]) {
		let count = self.nr_block_in_group();
		let base = 1024 / self.block_size().as_bytes();

		// pr_debug!(
		// 	"sb: info: backup_bid: {} {}",
		// 	count,
		// 	self.block_size().as_bytes()
		// );

		for (i, b) in buf.iter_mut().enumerate() {
			*b = base + (2 * i).checked_sub(1).unwrap_or_default() * count;
		}
	}

	#[inline]
	pub fn uuid(&self) -> &[u8] {
		&self.uuid
	}

	pub fn edit_for_mount(&mut self) {
		let sec = get_timestamp_second() as u32;
		self.state = FsState::Error as u16;
		self.wtime = sec;
		self.mtime = sec;
		self.mnt_count += 1;
	}

	pub fn edit_for_unmount(&mut self) {
		let sec = get_timestamp_second() as u32;
		self.state = FsState::Valid as u16;
		self.wtime = sec;
	}

	#[inline]
	pub fn dec_free_inodes_count(&mut self, count: usize) {
		self.free_inodes_count -= count as u32;
	}

	#[inline]
	pub fn inc_free_inodes_count(&mut self, count: usize) {
		self.free_inodes_count += count as u32;
	}

	#[inline]
	pub fn dec_free_blocks_count(&mut self, count: usize) {
		self.free_blocks_count -= count as u32;
	}

	#[inline]
	pub fn inc_free_blocks_count(&mut self, count: usize) {
		self.free_blocks_count += count as u32;
	}

	#[inline]
	pub fn free_inodes_count(&self) -> usize {
		self.free_inodes_count as usize
	}

	pub fn free_blocks_count(&self) -> usize {
		let current = unsafe { CURRENT.get_ref() };

		if current.is_privileged() {
			self.free_blocks_count as usize
		} else {
			(self.free_blocks_count - self.r_blocks_count) as usize
		}
	}
}

impl Debug for SuperBlockInfo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		Display::fmt(&self, f)
	}
}

impl Display for SuperBlockInfo {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "\n")?;
		write_field!(f, self, inodes_count)?;
		write_field!(f, self, blocks_count)?;
		write_field!(f, self, r_blocks_count)?;
		write_field!(f, self, free_blocks_count)?;
		write_field!(f, self, free_inodes_count)?;
		write_field!(f, self, first_data_block)?;
		write_field!(f, self, log_block_size)?;
		write_field!(f, self, log_frag_size)?;
		write_field!(f, self, blocks_per_group)?;
		write_field!(f, self, frags_per_group)?;
		write_field!(f, self, inodes_per_group)?;
		write_field!(f, self, mtime)?;
		write_field!(f, self, wtime)?;
		write_field!(f, self, mnt_count)?;
		write_field!(f, self, max_mnt_count)?;
		write_field!(f, self, magic)?;
		write_field!(f, self, state)?;
		write_field!(f, self, errors)?;
		write_field!(f, self, minor_rev_level)?;
		write_field!(f, self, lastcheck)?;
		write_field!(f, self, checkinterval)?;
		write_field!(f, self, creator_os)?;
		write_field!(f, self, rev_level)?;
		write_field!(f, self, def_resuid)?;
		write_field!(f, self, def_resgid)?;
		write_field!(f, self, first_ino)?;
		write_field!(f, self, inode_size)?;
		write_field!(f, self, block_group_nr)?;
		write_field!(f, self, feature_compat)?;
		write_field!(f, self, feature_incompat)?;
		write_field!(f, self, feature_ro_compat)?;
		write_field!(f, self, uuid)?;
		write_field!(f, self, volume_name)?;
		write_field!(f, self, last_mounted)?;
		write_field!(f, self, algo_bitmap)?;
		write_field!(f, self, prealloc_blocks)?;
		write_field!(f, self, prealloc_dir_blocks)?;

		Ok(())
	}
}
