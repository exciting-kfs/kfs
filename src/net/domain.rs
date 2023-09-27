use crate::syscall::errno::Errno;

use super::local::domain::LocalDomain;
use super::socket::{SocketHandle, SocketKind};

pub trait SocketDomain {
	fn create_socket(kind: SocketKind, protocol: i32) -> Result<SocketHandle, Errno>;
}

pub mod code {
	pub const LOCAL: i32 = 0;
}

pub fn create_socket(domain: i32, kind: SocketKind, protocol: i32) -> Result<SocketHandle, Errno> {
	match domain {
		code::LOCAL => LocalDomain::create_socket(kind, protocol),
		_ => Err(Errno::EINVAL),
	}
}
