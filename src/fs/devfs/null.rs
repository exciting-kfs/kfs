use alloc::boxed::Box;

use crate::{
	fs::vfs::{FileHandle, FileInode, IOFlag, Permission, RawStat, RealInode, TimeSpec, Whence},
	syscall::errno::Errno,
};

pub struct DevNull;

impl RealInode for DevNull {
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

impl FileInode for DevNull {
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
		Ok(Box::new(DevNull))
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

impl FileHandle for DevNull {
	fn read(&self, _buf: &mut [u8], _flags: IOFlag) -> Result<usize, Errno> {
		Ok(0)
	}

	fn write(&self, buf: &[u8], _flags: IOFlag) -> Result<usize, Errno> {
		Ok(buf.len())
	}

	fn lseek(&self, _offset: isize, _whence: Whence) -> Result<usize, Errno> {
		Ok(0)
	}
}
