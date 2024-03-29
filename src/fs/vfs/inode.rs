use alloc::{boxed::Box, sync::Arc};
use bitflags::bitflags;

use crate::{
	fs::{devfs::partition::DevPart, path::Path},
	sync::Locked,
	syscall::errno::Errno,
};

use super::{DirHandle, FileHandle, Statx, StatxMode, StatxTimeStamp, VfsEntry};

#[derive(Copy, Clone, Debug)]
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
	#[derive(Clone, Copy, Debug)]
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
	#[derive(Clone, Copy, Debug)]
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
	Socket(Arc<SocketInode>),
	Block(Arc<DevPart>),
}

#[repr(C)]
#[derive(Default, Clone)]
pub struct TimeSpec {
	seconds: isize,
	nanoseconds: isize,
}

impl TimeSpec {
	pub fn nano(&self) -> u64 {
		self.seconds as u64 * 1_000_000_000 + self.nanoseconds as u64
	}
}

impl From<u64> for TimeSpec {
	fn from(value: u64) -> Self {
		Self {
			seconds: (value / 1_000_000_000) as isize,
			nanoseconds: (value % 1_000_000_000) as isize,
		}
	}
}

impl Into<StatxTimeStamp> for TimeSpec {
	fn into(self) -> StatxTimeStamp {
		StatxTimeStamp {
			sec: self.seconds as i64,
			nsec: self.nanoseconds as u32,
			pad: 0,
		}
	}
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

pub trait Inode {
	fn stat(&self) -> Result<Statx, Errno>;
	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno>;
	fn chmod(&self, perm: Permission) -> Result<(), Errno>;
	fn access(&self, uid: usize, gid: usize, perm: Permission) -> Result<(), Errno> {
		let stat = self.stat()?;
		let file_perm = stat.get_perm();

		if !default_access(stat.uid, stat.gid, file_perm, uid, gid, perm) {
			return Err(Errno::EACCES);
		}

		Ok(())
	}
}

pub trait DirInode: Inode {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno>;
	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno>;
	fn mkdir(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn DirInode>, Errno>;
	fn rmdir(&self, name: &[u8]) -> Result<(), Errno>;
	fn create(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn FileInode>, Errno>;
	fn unlink(&self, name: &[u8]) -> Result<(), Errno>;
	fn symlink(&self, target: &[u8], name: &[u8]) -> Result<Arc<dyn SymLinkInode>, Errno>;
	fn link(&self, src: &VfsEntry, link_name: &[u8]) -> Result<VfsInode, Errno>;
	fn overwrite(&self, src: &VfsEntry, link_name: &[u8]) -> Result<VfsInode, Errno>;
}

pub trait FileInode: Inode {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno>;
	fn truncate(&self, length: isize) -> Result<(), Errno>;
}

pub trait SymLinkInode: Inode {
	fn target(&self) -> Result<Path, Errno>;
}

pub struct SocketInode {
	perm: Locked<Permission>,
	owner: Locked<usize>,
	group: Locked<usize>,
	atime: Locked<TimeSpec>,
	mtime: Locked<TimeSpec>,
	ctime: Locked<TimeSpec>,
}

impl SocketInode {
	pub fn new(perm: Permission, owner: usize, group: usize) -> Self {
		Self {
			perm: Locked::new(perm),
			owner: Locked::new(owner),
			group: Locked::new(group),
			atime: Locked::default(),
			mtime: Locked::default(),
			ctime: Locked::default(),
		}
	}
}

impl Inode for SocketInode {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: *self.owner.lock(),
			gid: *self.group.lock(),
			mode: StatxMode::new(StatxMode::SOCKET, self.perm.lock().bits() as u16),
			pad1: 0,
			ino: 0,
			size: 0,
			blocks: 0,
			attributes_mask: 0,
			atime: self.atime.lock().clone().into(),
			btime: StatxTimeStamp::default(),
			ctime: self.ctime.lock().clone().into(),
			mtime: self.mtime.lock().clone().into(),
			rdev_major: 0,
			rdev_minor: 0,
			dev_major: 0,
			dev_minor: 0,
		})
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		*self.owner.lock() = owner;
		*self.group.lock() = group;

		Ok(())
	}

	fn chmod(&self, perm: Permission) -> Result<(), Errno> {
		*self.perm.lock() = perm;

		Ok(())
	}
}
