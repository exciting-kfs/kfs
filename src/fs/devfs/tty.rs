use alloc::boxed::Box;

use crate::{
	driver::terminal::TTYFile,
	fs::vfs::{FileHandle, FileInode, Inode, Permission, Statx, StatxMode, StatxTimeStamp},
	process::task::CURRENT,
	syscall::errno::Errno,
};

pub struct DevTTY {
	inner: TTYFile,
}

impl DevTTY {
	pub fn new(inner: TTYFile) -> Self {
		Self { inner }
	}
}

impl Inode for DevTTY {
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

	fn chown(&self, _owner: usize, _groupp: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

impl FileInode for DevTTY {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
		let current = unsafe { CURRENT.get_mut() };

		if let Some(ref ext) = current.get_user_ext() {
			let sess = &ext.lock_relation().get_session();
			if let Ok(_) = self.inner.lock_tty().connect(sess) {
				sess.lock().set_ctty(self.inner.clone());
			}
		}

		Ok(Box::new(self.inner.clone()))
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}
