use alloc::{
	boxed::Box,
	sync::{Arc, Weak},
};
use kernel::{
	elf::kobject::KernelModule,
	fs::vfs::{FileHandle, FileInode, Inode, Permission, RawStat, TimeSpec},
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
	fn stat(&self) -> Result<RawStat, Errno> {
		Ok(RawStat {
			perm: 0o666,
			uid: 0,
			gid: 0,
			size: 0,
			file_type: 1,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
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
