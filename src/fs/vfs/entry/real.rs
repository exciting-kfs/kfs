use alloc::sync::Arc;

use crate::fs::vfs::RealInode;
use crate::{process::task::Task, syscall::errno::Errno};

use super::block::VfsBlockEntry;
use super::{
	AccessFlag, IOFlag, Permission, RawStat, VfsDirEntry, VfsFileEntry, VfsHandle, VfsSocketEntry,
};

use enum_dispatch::enum_dispatch;

#[enum_dispatch(RealEntry, Entry)]
#[derive(Clone)]
pub enum VfsRealEntry {
	File(Arc<VfsFileEntry>),
	Dir(Arc<VfsDirEntry>),
	Socket(Arc<VfsSocketEntry>),
	Block(Arc<VfsBlockEntry>),
}

impl VfsRealEntry {
	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(_) | Socket(_) | Block(_) => Err(Errno::ENOTDIR),
			Dir(d) => Ok(d),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => Ok(f),
			Dir(_) => Err(Errno::EISDIR),
			Socket(_) | Block(_) => Err(Errno::ESPIPE),
		}
	}

	pub fn downcast_socket(self) -> Result<Arc<VfsSocketEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(_) => Err(Errno::ECONNREFUSED),
			Dir(_) => Err(Errno::ECONNREFUSED),
			Block(_) => Err(Errno::ECONNREFUSED),
			Socket(s) => Ok(s),
		}
	}

	pub fn downcast_block(self) -> Result<Arc<VfsBlockEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(_) => Err(Errno::ESPIPE),
			Dir(_) => Err(Errno::EISDIR),
			Socket(_) => Err(Errno::ESPIPE),
			Block(b) => Ok(b),
		}
	}

	pub fn open(
		&self,
		io_flags: IOFlag,
		access_flags: AccessFlag,
		task: &Arc<Task>,
	) -> Result<VfsHandle, Errno> {
		let read_perm = access_flags
			.read_ok()
			.then_some(Permission::ANY_READ)
			.unwrap_or(Permission::empty());

		let write_perm = access_flags
			.write_ok()
			.then_some(Permission::ANY_WRITE)
			.unwrap_or(Permission::empty());

		let perm = read_perm | write_perm;
		self.access(perm, task)?;

		use VfsRealEntry::*;
		match self {
			File(f) => Ok(VfsHandle::File(f.open(io_flags, access_flags)?)),
			Dir(d) => Ok(VfsHandle::Dir(d.open(io_flags, access_flags)?)),
			Socket(_) => Err(Errno::ENOENT),
			Block(_) => Err(Errno::ENOENT), // TODO
		}
	}
}

#[enum_dispatch]
pub trait RealEntry {
	// TODO macro: why this can't return `&Arc<dyn RealInode>`?
	fn real_inode(&self) -> Arc<dyn RealInode>;

	fn stat(&self) -> Result<RawStat, Errno> {
		self.real_inode().stat()
	}

	fn access(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		self.real_inode()
			.access(task.get_uid(), task.get_gid(), perm)
	}

	fn chmod(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		let owner = self.stat()?.uid;

		let uid = task.get_uid();
		if uid != 0 && uid != owner {
			return Err(Errno::EPERM);
		}

		self.real_inode().chmod(perm)
	}

	fn chown(&self, owner: usize, group: usize, task: &Arc<Task>) -> Result<(), Errno> {
		if task.get_uid() != 0 {
			// TODO: group check
			return Err(Errno::EPERM);
		}

		self.real_inode().chown(owner, group)
	}
}
