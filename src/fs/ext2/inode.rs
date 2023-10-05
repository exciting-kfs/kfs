use alloc::{boxed::Box, sync::Arc};

use crate::{
	fs::vfs::{self, TimeSpec},
	sync::LockRW,
	syscall::errno::Errno,
};

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
pub struct Inum(usize);

impl Inum {
	pub fn new(num: usize) -> Option<Inum> {
		if num >= 1 {
			Some(Inum(num))
		} else {
			None
		}
	}

	pub unsafe fn new_unchecked(num: usize) -> Inum {
		Inum(num)
	}

	#[inline(always)]
	pub fn index(&self) -> usize {
		self.0 - 1
	}
}

#[derive(Debug)]
pub enum CastError {
	NotFile,
	NotDir,
}

pub struct Inode {
	info: InodeInfo,
}

impl Inode {
	pub fn new(info: InodeInfo) -> Self {
		Self { info }
	}
}

impl LockRW<Inode> {
	pub fn downcast_dir(self: Arc<Self>) -> Result<DirInode, CastError> {
		// TODO mode enum?
		let inode = self.read_lock();
		match inode.info.mode & 0xf000 {
			0x4000 => {
				drop(inode);
				Ok(DirInode(self))
			}
			_ => Err(CastError::NotDir),
		}
	}

	pub fn downcast_file(self: Arc<Self>) -> Result<FileInode, CastError> {
		let inode = self.read_lock();

		match inode.info.mode & 0xf000 {
			0x4000 => Err(CastError::NotDir),
			_ => {
				drop(inode);
				Ok(FileInode(self))
			}
		}
	}
}

pub struct DirInode(Arc<LockRW<Inode>>);

#[allow(unused)]
impl vfs::DirInode for DirInode {
	fn open(&self) -> Result<Box<dyn vfs::DirHandle>, Errno> {
		todo!()
	}
	fn stat(&self) -> Result<vfs::RawStat, Errno> {
		todo!()
	}

	fn chmod(&self, perm: vfs::Permission) -> Result<(), Errno> {
		todo!()
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		todo!()
	}

	fn lookup(&self, name: &[u8]) -> Result<vfs::VfsInode, Errno> {
		todo!()
	}

	fn symlink(&self, target: &[u8], name: &[u8]) -> Result<Arc<dyn vfs::SymLinkInode>, Errno> {
		todo!()
	}

	fn mkdir(&self, name: &[u8], perm: vfs::Permission) -> Result<Arc<dyn vfs::DirInode>, Errno> {
		todo!()
	}

	fn rmdir(&self, name: &[u8]) -> Result<(), Errno> {
		todo!()
	}

	fn create(&self, name: &[u8], perm: vfs::Permission) -> Result<Arc<dyn vfs::FileInode>, Errno> {
		todo!()
	}

	fn unlink(&self, name: &[u8]) -> Result<(), Errno> {
		todo!()
	}
}

pub struct FileInode(Arc<LockRW<Inode>>);

#[allow(unused)]
impl vfs::FileInode for FileInode {
	fn open(&self) -> Result<Box<dyn vfs::FileHandle>, Errno> {
		todo!()
	}

	fn stat(&self) -> Result<vfs::RawStat, Errno> {
		let inode = self.0.read_lock();

		let perm = (inode.info.mode & 0x0fff) as u32;
		let uid = inode.info.uid as usize;
		let gid = inode.info.gid as usize;
		// let size: u64 = (inode.info.dir_acl as u64) << 32 | inode.info.size as u64;
		let size = inode.info.size as isize;

		Ok(vfs::RawStat {
			perm,
			uid,
			gid,
			size,
			file_type: 1,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chmod(&self, perm: vfs::Permission) -> Result<(), Errno> {
		let mut inode = self.0.write_lock();

		let mode = inode.info.mode & 0xf000;
		let perm = perm.bits() as u16;

		inode.info.mode = mode | perm;

		Ok(())
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		let mut inode = self.0.write_lock();

		inode.info.uid = owner as u16;
		inode.info.gid = group as u16;

		Ok(())
	}

	fn truncate(&self, length: isize) -> Result<(), Errno> {
		todo!()
	}
}

#[derive(Clone, Debug)]
#[repr(C)]
pub struct InodeInfo {
	pub mode: u16,
	pub uid: u16,
	pub size: u32,
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
	pub faddr: u32,
	pub osd2: [u32; 3],
}
