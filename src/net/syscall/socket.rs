use alloc::sync::Arc;

use crate::fs::vfs::{AccessFlag, IOFlag, VfsHandle, VfsSocketHandle};
use crate::net::domain::create_socket;
use crate::net::socket::SocketKind;
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;

pub fn sys_socket(domain: i32, kind: i32, protocol: i32) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };
	let kind = SocketKind::from_raw(kind)?;

	let socket = create_socket(domain, kind, protocol)?;

	let handle = VfsHandle::Socket(Arc::new(VfsSocketHandle::new(
		None,
		socket,
		IOFlag::empty(),
		AccessFlag::O_RDWR,
	)));

	current
		.get_user_ext()
		.expect("must be user process")
		.lock_fd_table()
		.alloc_fd(handle)
		.ok_or(Errno::EMFILE)
		.map(|fd| fd.index())
}
