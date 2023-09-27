use crate::fs::vfs::VfsHandle;
use crate::net::address::UnknownSocketAddress;
use crate::process::{fd_table::Fd, task::CURRENT};
use crate::syscall::errno::Errno;

pub fn sys_connect(socket_fd: usize, addr: usize, addr_len: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let addr = UnknownSocketAddress::new_read_only(addr, addr_len, current)
		.and_then(|x| x.ok_or(Errno::EINVAL))?;

	let fd = Fd::from(socket_fd).ok_or(Errno::EBADF)?;

	let handle = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table()
		.get_file(fd)
		.ok_or(Errno::EBADF)
		.and_then(|h| match h {
			VfsHandle::Socket(s) => Ok(s),
			_ => Err(Errno::EBADF),
		})?;

	handle.connect(&addr, current).map(|_| 0)
}
