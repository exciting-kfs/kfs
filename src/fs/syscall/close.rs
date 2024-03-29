use crate::fs::delete_fd_node;
use crate::process::{fd_table::Fd, task::CURRENT};
use crate::syscall::errno::Errno;

pub fn sys_close(fildes: isize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };
	let ext = current.user_ext_ok_or(Errno::EPERM)?;

	let fd = Fd::from(fildes as usize).ok_or(Errno::EBADF)?;

	let handle = ext.lock_fd_table().close(fd.clone())?;

	handle.close()?;

	let _ = delete_fd_node(current.get_pid(), fd);

	Ok(0)
}
