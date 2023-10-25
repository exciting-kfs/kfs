use core::mem::{size_of, transmute};

use alloc::sync::Arc;
use kernel::{
	driver::hpet::get_timestamp_nano,
	elf::kobject::KernelModule,
	fs::vfs::{FileHandle, IOFlag, TimeSpec, Whence},
	syscall::errno::Errno,
};

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
		if buf.len() < size_of::<TimeSpec>() {
			return Err(Errno::EINVAL);
		}

		let time = TimeSpec::from(get_timestamp_nano());
		let src: [u8; size_of::<TimeSpec>()] = unsafe { transmute(time) };

		buf[..size_of::<TimeSpec>()].copy_from_slice(&src);

		Ok(size_of::<TimeSpec>())
	}

	fn write(&self, _buf: &[u8], _flags: IOFlag) -> Result<usize, Errno> {
		Err(Errno::EPERM)
	}
}
