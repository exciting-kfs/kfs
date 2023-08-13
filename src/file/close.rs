use crate::process::{fd_table::Fd, task::CURRENT};
use crate::syscall::errno::Errno;

pub fn sys_close(fildes: isize) -> Result<usize, Errno> {
	let ext = unsafe { CURRENT.get_mut() }.user_ext_ok_or(Errno::EPERM)?;

	let fd = Fd::from(fildes as usize).ok_or(Errno::EBADF)?;

	let mut fd_table = ext.lock_fd_table();

	fd_table.close(fd)
}
