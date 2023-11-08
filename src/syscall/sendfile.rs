use crate::{
	fs::vfs::Whence,
	process::{fd_table::Fd, task::CURRENT},
};

use super::errno::Errno;

pub fn sys_sendfile(
	out_fd: isize,
	in_fd: isize,
	offset: isize,
	count: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let fd_table = current.user_ext_ok_or(Errno::EPERM)?.lock_fd_table();

	let src = Fd::from(in_fd as usize)
		.and_then(|fd| fd_table.get_file(fd))
		.ok_or(Errno::EBADF)?;

	let dst = Fd::from(out_fd as usize)
		.and_then(|fd| fd_table.get_file(fd))
		.ok_or(Errno::EBADF)?;

	src.lseek(offset, Whence::Begin)?;

	todo!()
}
