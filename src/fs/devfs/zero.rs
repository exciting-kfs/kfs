use alloc::boxed::Box;

use crate::{
	fs::vfs::{FileHandle, FileInode, IOFlag, Permission, RawStat, TimeSpec, Whence},
	syscall::errno::Errno,
};

pub struct DevZero;

impl FileInode for DevZero {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
		Ok(Box::new(DevZero))
	}

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

	fn chown(&self, _owner: usize, _groupp: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

impl FileHandle for DevZero {
	fn read(&self, buf: &mut [u8], _flags: IOFlag) -> Result<usize, Errno> {
		buf.fill(0);

		Ok(buf.len())
	}

	fn write(&self, buf: &[u8], _flags: IOFlag) -> Result<usize, Errno> {
		Ok(buf.len())
	}

	fn lseek(&self, _offset: isize, _whence: Whence) -> Result<usize, Errno> {
		Ok(0)
	}
}
