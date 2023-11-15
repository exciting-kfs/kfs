use core::mem::size_of;
use core::slice::from_raw_parts;

use crate::mm::user::verify::{verify_buffer_mut, verify_region};
use crate::mm::user::vma::AreaFlag;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

use super::get_file;

pub fn sys_read(fd: isize, buf: usize, len: usize) -> Result<usize, Errno> {
	if len == 0 {
		return Ok(0);
	}

	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf, len, current)?;

	get_file(fd)?.read(buf)
}

#[repr(C)]
struct IOVec {
	base: usize,
	len: usize,
}

pub fn sys_readv(fd: isize, iov: usize, iovcnt: usize) -> Result<usize, Errno> {
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
		let curr = sys_read(fd, ent.base, ent.len)?;

		ret += curr;
	}

	Ok(ret)
}
