use core::{mem::size_of, slice::from_raw_parts};

use crate::{
	mm::user::{
		verify::{verify_buffer, verify_region},
		vma::AreaFlag,
	},
	process::task::CURRENT,
	syscall::errno::Errno,
};

use super::get_file;

pub fn sys_write(fd: isize, buf: usize, len: usize) -> Result<usize, Errno> {
	if len == 0 {
		return Ok(0);
	}

	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer(buf, len, current)?;

	get_file(fd)?.write(buf)
}

#[repr(C)]
struct IOVec {
	base: usize,
	len: usize,
}

pub fn sys_writev(fd: isize, iov: usize, iovcnt: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	verify_region(
		iov,
		iovcnt * size_of::<IOVec>(),
		current,
		AreaFlag::Readable,
	)?;

	let iov = unsafe { from_raw_parts(iov as *const IOVec, iovcnt) };

	let mut ret = 0;
	for ent in iov {
		ret += sys_write(fd, ent.base, ent.len)?;
	}

	Ok(ret)
}
