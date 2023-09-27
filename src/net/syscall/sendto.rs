use crate::fs::syscall::get_file;
use crate::fs::vfs::VfsHandle;
use crate::mm::user::verify::verify_buffer;
use crate::net::address::UnknownSocketAddress;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_sendto(
	socket_fd: isize,
	buf: usize,
	buf_len: usize,
	addr: usize,
	addr_len: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer(buf, buf_len, current)?;
	let addr = UnknownSocketAddress::new_read_only(addr, addr_len, current)?;

	use VfsHandle::*;
	let handle = match get_file(socket_fd)? {
		Socket(x) => Ok(x),
		File(_) | Dir(_) => Err(Errno::EBADF),
	}?;

	handle.send_to(&addr, buf)
}
