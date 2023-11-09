use core::{
	ops::{Deref, DerefMut},
	ptr::copy_nonoverlapping,
};

use crate::{
	driver::{
		hpet::{get_timestamp_nano, get_timestamp_second},
		partition::BlockId,
	},
	fs::vfs::{self, FileType, Permission},
	mm::constant::SECTOR_SIZE,
	process::task::CURRENT,
	sync::{ReadLockGuard, WriteLockGuard},
	syscall::errno::Errno,
};

use super::{data::DataWrite, Inode};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct InodeInfo {
	pub mode: u16,
	pub uid: u16,
	size: u32,
	pub atime: u32,
	pub ctime: u32,
	pub mtime: u32,
	pub dtime: u32,
	pub gid: u16,
	pub links_count: u16,
	pub blocks: u32,
	pub flags: u32,
	pub osd1: u32,
	pub block: [u32; 15],
	pub generation: u32,
	pub file_acl: u32,
	pub dir_acl: u32,
	pub faddr: u32, // ?
	pub osd2: [u32; 3],
}

impl InodeInfo {
	pub fn new(file_type: FileType, perm: Permission) -> Self {
		let current = unsafe { CURRENT.get_mut() };

		let gid = current.get_gid() as u16;
		let uid = current.get_uid() as u16;
		let timestamp = get_timestamp_second() as u32;
		let mode = file_type.mode() | perm.bits() as u16;
		let links_count = match file_type {
			FileType::Directory => 2,
			_ => 1,
		};

		Self {
			mode,
			uid,
			size: 0,
			atime: timestamp,
			ctime: timestamp,
			mtime: timestamp,
			dtime: 0,
			gid,
			links_count,
			blocks: 0,
			flags: 0,
			osd1: 0,
			block: [0; 15],
			generation: 0,
			file_acl: 0,
			dir_acl: 0,
			faddr: 0,
			osd2: [0; 3],
		}
	}

	pub fn clone_for_delete(&self) -> Self {
		let mut info = self.clone();

		info.block[0] = 0;
		info.size = 0;
		info.blocks = 0;
		info.dtime = (get_timestamp_nano() / 1 << 30) as u32;
		info
	}

	#[inline]
	pub fn get_size(&self) -> usize {
		// (inode.info.dir_acl as u64) << 32 | inode.info.size as u64
		self.size as usize
	}

	#[inline]
	pub fn set_size(&mut self, size: usize) {
		// self.dir_acl = (size & 0xffff_ffff_0000_0000) >> 32;
		self.size = size as u32;
	}

	#[inline]
	pub fn end_of_blocks(&self) -> usize {
		(self.blocks as usize) * SECTOR_SIZE
	}

	pub fn bid_array(&self, index: usize) -> Option<BlockId> {
		if index >= 15 {
			None
		} else {
			let bid = unsafe { BlockId::new_unchecked(self.block[index] as usize) };
			Some(bid)
		}
	}

	pub fn write(&mut self, other: &Self) {
		unsafe { copy_nonoverlapping(other, self, 1) };
	}

	pub fn inc_blocks(&mut self, delta_bytes: usize) {
		let blocks = self.blocks as usize + delta_bytes / SECTOR_SIZE;

		self.blocks = blocks as u32;
	}

	pub fn dec_blocks(&mut self, delta_bytes: usize) {
		let blocks = self.blocks as usize - delta_bytes / SECTOR_SIZE;

		self.blocks = blocks as u32;
	}

	pub fn chmod(&mut self, perm: vfs::Permission) -> Result<(), Errno> {
		let mode = self.mode & 0xf000;
		let perm = perm.bits() as u16;

		self.mode = mode | perm;

		Ok(())
	}

	pub fn chown(&mut self, owner: usize, group: usize) -> Result<(), Errno> {
		self.uid = owner as u16;
		self.gid = group as u16;

		Ok(())
	}

	pub fn is_unique(&self) -> bool {
		use FileType::*;
		match FileType::from_mode(self.mode) {
			Directory => self.links_count == 2,
			_ => self.links_count == 1,
		}
	}
}

pub struct InodeInfoRef<'a> {
	inode: ReadLockGuard<'a, Inode>,
}

impl<'a> InodeInfoRef<'a> {
	pub fn new(inode: ReadLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}
}

impl<'a> Deref for InodeInfoRef<'a> {
	type Target = InodeInfo;
	fn deref(&self) -> &Self::Target {
		&self.inode.info
	}
}

pub struct InodeInfoMut<'a> {
	inode: WriteLockGuard<'a, Inode>,
}

impl<'a> InodeInfoMut<'a> {
	pub fn new(inode: WriteLockGuard<'a, Inode>) -> Self {
		Self { inode }
	}

	pub fn from_data(data: DataWrite<'a>) -> Self {
		let inode = data.destruct();
		Self { inode }
	}
}

impl<'a> Drop for InodeInfoMut<'a> {
	fn drop(&mut self) {
		self.inode.dirty();
	}
}

impl<'a> Deref for InodeInfoMut<'a> {
	type Target = InodeInfo;
	fn deref(&self) -> &Self::Target {
		&self.inode.info
	}
}

impl<'a> DerefMut for InodeInfoMut<'a> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.inode.info
	}
}
