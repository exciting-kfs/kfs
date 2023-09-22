use crate::fs::vfs::VfsHandle;
use crate::process::{fd_table::Fd, task::CURRENT};
use crate::syscall::errno::Errno;

use super::utils::verify_buffer_mut;

pub(super) fn get_file(fd: isize) -> Result<VfsHandle, Errno> {
	let fd = Fd::from(fd as usize).ok_or(Errno::EBADF)?;
	let fd_table = unsafe { CURRENT.get_mut() }
		.get_user_ext()
		.expect("user task")
		.lock_fd_table();

	fd_table.get_file(fd).ok_or(Errno::EBADF)
}

pub fn sys_read(fd: isize, buf: usize, len: usize) -> Result<usize, Errno> {
	if len == 0 {
		return Ok(0);
	}

	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf, len, current)?;

	get_file(fd)?.read(buf)
}
