use alloc::sync::Arc;

use crate::fs::vfs::VfsHandle;
use crate::mm::user::verify::verify_ptr_mut;
use crate::net::address::UnknownSocketAddress;
use crate::process::{fd_table::Fd, task::CURRENT};
use crate::syscall::errno::Errno;

pub fn sys_accept(socket_fd: usize, addr: usize, addr_len_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let mut addr = match addr_len_ptr == 0 {
		true => None,
		false => {
			let addr_len = verify_ptr_mut::<usize>(addr_len_ptr, current)?;

			UnknownSocketAddress::new_write_only(addr, *addr_len, current)?
		}
	};

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

	let vfs_socket_handle = socket_handle.accept(&mut addr)?;

	if let Some(ref addr) = addr {
		unsafe { *(addr_len_ptr as *mut usize) = addr.len };
	}

	let fd = current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table()
		.alloc_fd(VfsHandle::Socket(Arc::new(vfs_socket_handle)))
		.ok_or(Errno::EMFILE)?;

	Ok(fd.index())
}
