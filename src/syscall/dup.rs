use crate::process::{fd_table::Fd, task::CURRENT};

use super::errno::Errno;

pub fn sys_dup2(fd1: usize, fd2: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let fd1 = Fd::from(fd1).ok_or(Errno::EBADF)?;
	let fd2 = Fd::from(fd2).ok_or(Errno::EBADF)?;

	let mut fd_table = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table();

	let old_handle = fd_table.dup2(fd1, fd2)?;

	if let Some(old) = old_handle {
		old.close()?;
	}

	Ok(0)
}

pub fn sys_dup(fd: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let fd = Fd::from(fd).ok_or(Errno::EBADF)?;

	let mut fd_table = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table();

	fd_table.dup(fd).map(|fd| fd.index())
}
