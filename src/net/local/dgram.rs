use alloc::{collections::LinkedList, sync::Arc, vec::Vec};

use crate::fs::vfs::{lookup_entry_follow, IOFlag, VfsSocketHandle};
use crate::net::address::{ReadOnly, UnknownSocketAddress, WriteOnly};
use crate::net::socket::{Socket, SocketHandle};
use crate::process::{signal::poll_signal_queue, task::Task};
use crate::scheduler::context::yield_now;
use crate::sync::Locked;
use crate::syscall::errno::Errno;

use super::address::{BindAddress, LocalSocketAddress};

struct LocalDgramPacket {
	pub sender: Option<LocalSocketAddress>,
	pub data: Vec<u8>,
}

pub struct LocalDgramSocket {
	peer: Locked<Option<Arc<LocalDgramSocket>>>,
	address: BindAddress,
	recv_buffer: Locked<LinkedList<LocalDgramPacket>>,
}

impl LocalDgramSocket {
	pub fn new() -> Self {
		Self {
			peer: Locked::new(None),
			address: BindAddress::new(),
			recv_buffer: Locked::new(LinkedList::new()),
		}
	}

	fn send_to_peer(
		&self,
		buf: &[u8],
		peer: Arc<LocalDgramSocket>,
		_io_flag: IOFlag,
	) -> Result<usize, Errno> {
		let address = self.address.clone_address();

		peer.recv_buffer.lock().push_back(LocalDgramPacket {
			sender: address,
			data: buf.to_vec(),
		});

		Ok(buf.len())
	}

	fn recv_packet(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		dst: &mut [u8],
		packet: &LocalDgramPacket,
	) -> Result<usize, Errno> {
		let size = dst.len().min(packet.data.len());

		(dst[..size]).copy_from_slice(&packet.data[..size]);

		if let Some(addr) = addr {
			addr.copy_from(&packet.sender)?;
		}

		Ok(size)
	}
}

impl Socket for LocalDgramSocket {
	fn send_to(
		&self,
		addr: &Option<UnknownSocketAddress<ReadOnly>>,
		buf: &[u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno> {
		let peer = match addr {
			Some(addr) => {
				if self.peer.lock().is_some() {
					Err(Errno::EISCONN)
				} else {
					let local_addr: LocalSocketAddress = addr.copy_to_solid()?;
					let socket = local_addr.lookup_socket()?;

					if let SocketHandle::LocalDgram(sock) = socket {
						Ok(sock)
					} else {
						Err(Errno::EPROTOTYPE)
					}
				}
			}
			None => self.peer.lock().clone().ok_or(Errno::ENOTCONN),
		}?;

		self.send_to_peer(buf, peer, io_flag)
	}

	fn recv_from(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		buf: &mut [u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno> {
		loop {
			let packet = self.recv_buffer.lock().pop_front();

			if let Some(ref packet) = packet {
				return self.recv_packet(addr, buf, packet);
			} else {
				unsafe { poll_signal_queue()? };

				if io_flag.contains(IOFlag::O_NONBLOCK) {
					return Err(Errno::EAGAIN);
				}

				yield_now();
				continue;
			}
		}
	}

	fn bind(
		&self,
		addr: &UnknownSocketAddress<ReadOnly>,
		handle: &Arc<VfsSocketHandle>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		self.address.bind(addr, handle, task)
	}

	fn listen(&self, _bakclog: usize) -> Result<(), Errno> {
		Err(Errno::EOPNOTSUPP)
	}

	fn accept(
		&self,
		_addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Result<VfsSocketHandle, Errno> {
		Err(Errno::EOPNOTSUPP)
	}

	fn connect(
		self: &Arc<Self>,
		addr: &UnknownSocketAddress<ReadOnly>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		let addr: LocalSocketAddress = addr.copy_to_solid()?;

		let socket = lookup_entry_follow(&addr.path, task)
			.and_then(|ent| ent.downcast_socket())
			.and_then(|sock_ent| sock_ent.get_socket())?;

		use SocketHandle::*;
		let socket = match socket.expose_socket() {
			LocalDgram(x) => x,
			_ => return Err(Errno::EPROTOTYPE),
		};

		self.peer.lock().replace(socket.clone());

		Ok(())
	}
}
