use alloc::{
	boxed::Box,
	sync::{Arc, Weak},
};
use kernel::{
	elf::kobject::KernelModule,
	fs::vfs::{FileHandle, FileInode, Inode, Permission, Statx, StatxMode, StatxTimeStamp},
	syscall::errno::Errno,
};

use crate::handle::TimestampHandle;

pub(crate) struct TimestampInode {
	module: Weak<KernelModule>,
}

impl TimestampInode {
	pub fn new(module: &Arc<KernelModule>) -> Self {
		Self {
			module: Arc::downgrade(&module),
		}
	}
}

impl Inode for TimestampInode {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: 0,
			gid: 0,
			mode: StatxMode::new(StatxMode::CHARDEV, 0o666),
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

impl FileInode for TimestampInode {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
		let module = self.module.upgrade().ok_or(Errno::ENODEV)?;

		Ok(Box::new(TimestampHandle::new(&module)))
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}
