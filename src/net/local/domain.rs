use alloc::sync::Arc;

use crate::net::socket::SocketHandle;
use crate::net::{domain::SocketDomain, socket::SocketKind};
use crate::syscall::errno::Errno;

use super::{dgram::LocalDgramSocket, stream::LocalStreamSocket};

pub struct LocalDomain;

impl SocketDomain for LocalDomain {
	fn create_socket(kind: SocketKind, protocol: i32) -> Result<SocketHandle, Errno> {
		if protocol != 0 {
			return Err(Errno::EINVAL);
		}

		let handle = match kind {
			SocketKind::Stream => {
				SocketHandle::LocalStream(Arc::new(LocalStreamSocket::new_unbound()))
			}
			SocketKind::Dgram => SocketHandle::LocalDgram(Arc::new(LocalDgramSocket::new())),
		};

		Ok(handle)
	}
}
