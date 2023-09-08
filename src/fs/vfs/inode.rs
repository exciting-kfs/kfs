use alloc::{boxed::Box, sync::Arc};
use bitflags::bitflags;

use crate::{fs::path::Path, syscall::errno::Errno};

use super::{DirHandle, FileHandle};

pub struct AccessFlag(i32);
impl AccessFlag {
	const RDONLY: i32 = 0o0;
	const WRONLY: i32 = 0o1;
	const RDWR: i32 = 0o2;
	const ACCESS: i32 = 0o3;

	pub const O_RDONLY: Self = AccessFlag(Self::RDONLY);
	pub const O_WRONLY: Self = AccessFlag(Self::WRONLY);
	pub const O_RDWR: Self = AccessFlag(Self::RDWR);

	pub fn from_bits_truncate(bits: i32) -> Self {
		Self(bits & Self::ACCESS)
	}

	pub fn read_ok(&self) -> bool {
		self.0 != Self::WRONLY
	}

	pub fn write_ok(&self) -> bool {
		self.0 != Self::RDONLY
	}

	pub fn bits(&self) -> i32 {
		self.0
	}
}

bitflags! {
	#[derive(Clone, Copy)]
	pub struct CreationFlag: i32 {
		const O_CREAT = 0o100;
		const O_EXCL = 0o200;
		const O_NOCTTY = 0o400;
		const O_TRUNC = 0o1000;
		const O_DIRECTORY = 0o200000;
		const O_NOFOLLOW = 0o400000;
		const O_CLOEXEC = 0o2000000;
	}
}

bitflags! {
	#[derive(Clone, Copy)]
	pub struct IOFlag: i32 {
		const O_APPEND = 0o2000;
		const O_NONBLOCK = 0o4000;
		const O_SYNC = 0o10000;
	}
}

bitflags! {
	#[derive(Clone, Copy, PartialEq, Eq)]
	pub struct Permission: u32 {
		const S_ISUID = 0o4000;
		const S_ISGID = 0o2000;
		const S_ISVTX = 0o1000;
		const S_IRUSR = 0o0400;
		const S_IWUSR = 0o0200;
		const S_IXUSR = 0o0100;
		const S_IRGRP = 0o0040;
		const S_IWGRP = 0o0020;
		const S_IXGRP = 0o0010;
		const S_IROTH = 0o0004;
		const S_IWOTH = 0o0002;
		const S_IXOTH = 0o0001;
	}
}

impl Permission {
	pub const OWNER: Self = Self::S_IRUSR.union(Self::S_IWUSR).union(Self::S_IXUSR);
	pub const GROUP: Self = Self::S_IRGRP.union(Self::S_IWGRP).union(Self::S_IXGRP);
	pub const OTHER: Self = Self::S_IROTH.union(Self::S_IWOTH).union(Self::S_IXOTH);
	pub const ANY_READ: Self = Self::S_IRUSR.union(Self::S_IRGRP).union(Self::S_IROTH);
	pub const ANY_WRITE: Self = Self::S_IWUSR.union(Self::S_IWGRP).union(Self::S_IWOTH);
	pub const ANY_EXECUTE: Self = Self::S_IXUSR.union(Self::S_IXGRP).union(Self::S_IXOTH);

	pub fn group_ok(self, target: Self) -> bool {
		let group = self & Self::GROUP;
		let target_group = target & Self::GROUP;

		(group | target_group) == group
	}

	pub fn owner_ok(self, target: Self) -> bool {
		let user = self & Self::OWNER;
		let target_user = target & Self::OWNER;

		(user | target_user) == user
	}

	pub fn other_ok(self, target: Self) -> bool {
		let other = self & Self::OTHER;
		let target_other = target & Self::OTHER;

		(other | target_other) == other
	}
}

#[derive(Clone)]
pub enum VfsInode {
	File(Arc<dyn FileInode>),
	Dir(Arc<dyn DirInode>),
	SymLink(Arc<dyn SymLinkInode>),
}

#[repr(C)]
pub struct RawStat {
	pub perm: u32,
	pub uid: usize,
	pub gid: usize,
	pub size: isize,
}

fn default_access(
	file_uid: usize,
	file_gid: usize,
	file_perm: Permission,
	req_uid: usize,
	req_gid: usize,
	req_perm: Permission,
) -> bool {
	if file_uid == req_uid && file_perm.owner_ok(req_perm) {
		return true;
	}

	if file_gid == req_gid && file_perm.group_ok(req_perm) {
		return true;
	}

	if file_perm.other_ok(req_perm) {
		return true;
	}

	return false;
}

pub enum CachePolicy {
	Never,
	Always,
}

pub trait DirInode {
	fn open(&self) -> Box<dyn DirHandle>;
	fn stat(&self) -> Result<RawStat, Errno>;
	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno>;
	fn chmod(&self, perm: Permission) -> Result<(), Errno>;
	fn access(&self, uid: usize, gid: usize, perm: Permission) -> Result<(), Errno> {
		let stat = self.stat()?;
		let file_perm = Permission::from_bits_truncate(stat.perm);

		if !default_access(stat.uid, stat.gid, file_perm, uid, gid, perm) {
			return Err(Errno::EACCES);
		}

		Ok(())
	}
	fn lookup(&self, name: &[u8]) -> Result<(CachePolicy, VfsInode), Errno>;
	fn mkdir(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn DirInode>, Errno>;
	fn rmdir(&self, name: &[u8]) -> Result<(), Errno>;
	fn create(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn FileInode>, Errno>;
	fn unlink(&self, name: &[u8]) -> Result<(), Errno>;
	fn symlink(&self, target: &[u8], name: &[u8]) -> Result<Arc<dyn SymLinkInode>, Errno>;
}

pub trait FileInode {
	fn open(&self) -> Box<dyn FileHandle>;
	fn stat(&self) -> Result<RawStat, Errno>;
	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno>;
	fn chmod(&self, perm: Permission) -> Result<(), Errno>;
	fn access(&self, uid: usize, gid: usize, perm: Permission) -> Result<(), Errno> {
		let stat = self.stat()?;
		let file_perm = Permission::from_bits_truncate(stat.perm);

		if !default_access(stat.uid, stat.gid, file_perm, uid, gid, perm) {
			return Err(Errno::EACCES);
		}

		Ok(())
	}
	fn truncate(&self, length: isize) -> Result<(), Errno>;
}

pub trait SymLinkInode {
	fn target(&self) -> &Path;
}
