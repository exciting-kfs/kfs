use core::slice::from_raw_parts_mut;

use alloc::sync::Arc;

use crate::{
	file::File,
	interrupt::syscall::errno::Errno,
	process::{fd_table::Fd, task::CURRENT},
};

pub(super) fn get_file(fd: isize) -> Result<Arc<File>, Errno> {
	let fd = Fd::from(fd as usize).ok_or(Errno::EBADF)?;
	let fd_table = unsafe { CURRENT.get_mut() }
		.fd_table
		.as_ref()
		.expect("user task");
	fd_table.get_file(fd).ok_or(Errno::EBADF)
}

// TODO copy to user..?
pub fn sys_read(fd: isize, buf: *mut u8, len: usize) -> Result<usize, Errno> {
	let file = get_file(fd)?;
	let buf = unsafe { from_raw_parts_mut(buf, len) };
	file.ops.read(&file, buf)
}
