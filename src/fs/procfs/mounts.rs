use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
	fs::vfs::{FileHandle, FileInode, Inode, Permission, Statx, StatxMode, StatxTimeStamp},
	sync::LocalLocked,
	syscall::errno::Errno,
	util::from_utf8_or,
};

use super::{ProcFileHandle, PROCFS_ROOT_DIR};

pub fn create_mount_entry(dev_path: &[u8], mount_point: &[u8], fs_name: &[u8]) {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let mounts = procfs.get_mounts();

	mounts
		.lock()
		.entries
		.push(MountInfo::new(dev_path, mount_point, fs_name));
}

pub fn delete_mount_entry(mount_point: &[u8]) -> Result<(), Errno> {
	let mount_point = from_utf8_or(mount_point, "none");
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let mounts = procfs.get_mounts();
	let mut mounts_lock = mounts.lock();

	let idx = mounts_lock
		.entries
		.iter()
		.rposition(|x| x.get_mount_point() == mount_point)
		.ok_or(Errno::ENOENT)?;

	mounts_lock.entries.remove(idx);

	Ok(())
}

struct MountInfo {
	dev_path: (usize, usize),
	mount_point: (usize, usize),
	fs_name: (usize, usize),
	contents: String,
}

impl MountInfo {
	pub fn new(dev_path: &[u8], mount_point: &[u8], fs_name: &[u8]) -> Self {
		let dev_path = from_utf8_or(dev_path, "none");
		let mount_point = from_utf8_or(mount_point, "none");
		let fs_name = from_utf8_or(fs_name, "none");

		let mut offset;
		let mut contents = String::new();

		offset = contents.len();
		contents.push_str(dev_path);
		let dev_path = (offset, contents.len());
		contents.push(' ');

		offset = contents.len();
		contents.push_str(mount_point);
		let mount_point = (offset, contents.len());
		contents.push(' ');

		offset = contents.len();
		contents.push_str(fs_name);
		let fs_name = (offset, contents.len());
		contents.push(' ');

		contents.push_str("defaults 0 0\n");

		Self {
			dev_path,
			mount_point,
			fs_name,
			contents,
		}
	}

	pub fn get_mount_point(&self) -> &str {
		let (start, end) = self.mount_point;

		&self.contents[start..end]
	}

	pub fn get_contents(&self) -> &str {
		&self.contents
	}
}

pub struct ProcMountsInode {
	entries: Vec<MountInfo>,
}

impl ProcMountsInode {
	pub fn new() -> Self {
		Self {
			entries: Vec::new(),
		}
	}
}

impl Inode for LocalLocked<ProcMountsInode> {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: 0,
			gid: 0,
			mode: StatxMode::new(StatxMode::REGULAR, 0o444),
			pad1: 0,
			ino: 0,
			size: 0,
			blocks: 0,
			attributes_mask: 0,
			atime: StatxTimeStamp::default(),
			btime: StatxTimeStamp::default(),
			ctime: StatxTimeStamp::default(),
			mtime: StatxTimeStamp::default(),
			rdev_major: 0,
			rdev_minor: 0,
			dev_major: 0,
			dev_minor: 0,
		})
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

impl FileInode for LocalLocked<ProcMountsInode> {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
		let contents = self
			.lock()
			.entries
			.iter()
			.fold(String::new(), |acc, cur| acc + cur.get_contents());
		Ok(Box::new(LocalLocked::new(ProcFileHandle::new(
			contents.into_bytes(),
		))))
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}
