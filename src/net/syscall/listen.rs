use crate::{
	fs::vfs::VfsHandle,
	process::{fd_table::Fd, task::CURRENT},
	syscall::errno::Errno,
};

pub fn sys_listen(socket_fd: usize, backlog: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let fd = Fd::from(socket_fd).ok_or(Errno::EBADF)?;

	let socket_handle = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table()
		.get_file(fd)
		.ok_or(Errno::EBADF)
		.and_then(|h| match h {
			VfsHandle::Socket(s) => Ok(s),
			_ => Err(Errno::ECONNREFUSED),
		})?;

	socket_handle.listen(backlog).map(|_| 0)
}
