use crate::syscall::errno::Errno;

use super::{socket::SocketHandle, socket::SocketKind};

pub trait SocketDomain {
	fn create_socket(kind: SocketKind, protocol: i32) -> Result<SocketHandle, Errno>;
}

pub fn create_socket(domain: i32, kind: SocketKind, protocol: i32) -> Result<SocketHandle, Errno> {
	match domain {
		_ => Err(Errno::EINVAL),
	}
}
