use core::mem::{size_of, transmute};

use alloc::sync::Arc;
use kernel::{
	elf::kobject::KernelModule,
	fs::vfs::{FileHandle, IOFlag, Whence},
	syscall::errno::Errno,
};

use crate::TimeVal;

#[allow(dead_code)]
pub(crate) struct TimestampHandle {
	module: Arc<KernelModule>,
}

impl TimestampHandle {
	pub fn new(module: &Arc<KernelModule>) -> Self {
		Self {
			module: module.clone(),
		}
	}
}

impl FileHandle for TimestampHandle {
	fn lseek(&self, _offset: isize, _whence: Whence) -> Result<usize, Errno> {
		Err(Errno::EPERM)
	}

	fn read(&self, buf: &mut [u8], _flags: IOFlag) -> Result<usize, Errno> {
		if buf.len() < size_of::<TimeVal>() {
			return Err(Errno::EINVAL);
		}

		let val: [u8; size_of::<TimeVal>()] = unsafe { transmute(TimeVal::current()) };

		buf[..size_of::<TimeVal>()].copy_from_slice(&val);

		Ok(size_of::<TimeVal>())
	}

	fn write(&self, _buf: &[u8], _flags: IOFlag) -> Result<usize, Errno> {
		Err(Errno::EPERM)
	}
}
