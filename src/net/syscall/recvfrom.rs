use crate::fs::syscall::get_file;
use crate::fs::vfs::VfsHandle;
use crate::mm::user::verify::{verify_buffer_mut, verify_ptr_mut};
use crate::net::address::UnknownSocketAddress;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_recvfrom(
	socket_fd: isize,
	buf: usize,
	buf_len: usize,
	addr: usize,
	addr_len_ptr: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_buffer_mut(buf, buf_len, current)?;
	let mut addr = match addr_len_ptr == 0 {
		true => None,
		false => {
			let addr_len = verify_ptr_mut::<usize>(addr_len_ptr, current)?;

			UnknownSocketAddress::new_write_only(addr, *addr_len, current)?
		}
	};

	use VfsHandle::*;
	let handle = match get_file(socket_fd)? {
		Socket(x) => Ok(x),
		File(_) | Dir(_) => Err(Errno::EBADF),
	}?;

	let nread = handle.recv_from(&mut addr, buf)?;

	if let Some(ref addr) = addr {
		unsafe { *(addr_len_ptr as *mut usize) = addr.len };
	}

	Ok(nread)
}
