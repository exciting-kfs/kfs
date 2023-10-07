use alloc::collections::LinkedList;
use alloc::sync::{Arc, Weak};

use crate::collection::WrapQueue;
use crate::fs::vfs::{lookup_entry_follow, AccessFlag, IOFlag, VfsRealEntry, VfsSocketHandle};
use crate::net::address::{ReadOnly, UnknownSocketAddress, WriteOnly};
use crate::net::socket::{Socket, SocketHandle};
use crate::scheduler::context::yield_now;
use crate::sync::LocalLocked;
use crate::{process::task::Task, syscall::errno::Errno};

use super::address::{BindAddress, LocalSocketAddress};

enum StreamSocket {
	Created(BindAddress),
	Connected(Arc<ConnectedSocket>),
	Listening(ListeningSocket),
}

pub struct LocalStreamSocket {
	socket: LocalLocked<StreamSocket>,
}

impl LocalStreamSocket {
	pub fn new_unbound() -> Self {
		Self {
			socket: LocalLocked::new(StreamSocket::Created(BindAddress::new())),
		}
	}

	fn new(socket: StreamSocket) -> Self {
		Self {
			socket: LocalLocked::new(socket),
		}
	}

	fn make_connection(
		&self,
		out_addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Result<Arc<ConnectedSocket>, Errno> {
		let mut socket = self.socket.lock_check_signal()?;

		let addr = match &*socket {
			StreamSocket::Created(x) => Ok(x.take()),
			_ => Err(Errno::EINVAL),
		}?;

		if let Some(out_addr) = out_addr {
			if let Err(e) = out_addr.copy_from(&addr.clone_address()) {
				*socket = StreamSocket::Created(addr);
				return Err(e);
			}
		}

		let (me, peer) = ConnectedSocket::new(addr);

		*socket = StreamSocket::Connected(me);

		Ok(peer)
	}

	fn push_recv_buffer(&self, new_sock: &Arc<LocalStreamSocket>) -> Result<(), Errno> {
		let sock = self.socket.lock_check_signal()?;
		let sock = match &*sock {
			StreamSocket::Created(_) => Err(Errno::EINVAL),
			StreamSocket::Connected(_) => Err(Errno::EINVAL),
			StreamSocket::Listening(x) => Ok(x),
		}?;

		let mut accept_buffer = sock.accept_buffer.lock();
		if sock.backlog <= accept_buffer.len() {
			return Err(Errno::ECONNREFUSED);
		}

		accept_buffer.push_back(Arc::downgrade(new_sock));

		Ok(())
	}
}

impl Socket for LocalStreamSocket {
	fn send_to(
		&self,
		addr: &Option<UnknownSocketAddress<ReadOnly>>,
		buf: &[u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno> {
		let socket = self.socket.lock_check_signal()?;

		use StreamSocket::*;
		match *socket {
			Connected(ref x) => x.send_to(addr, buf, io_flag),
			Created(_) => Err(Errno::ENOTCONN),
			Listening(_) => Err(Errno::EOPNOTSUPP),
		}
	}

	fn recv_from(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		buf: &mut [u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno> {
		let socket = self.socket.lock_check_signal()?;

		use StreamSocket::*;
		match *socket {
			Connected(ref x) => x.recv_from(addr, buf, io_flag),
			Created(_) => Err(Errno::ENOTCONN),
			Listening(_) => Err(Errno::EOPNOTSUPP),
		}
	}

	fn bind(
		&self,
		addr: &UnknownSocketAddress<ReadOnly>,
		handle: &Arc<VfsSocketHandle>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		let socket = self.socket.lock_check_signal()?;

		use StreamSocket::*;
		match *socket {
			Created(ref address) => address.bind(addr, handle, task),
			Connected(ref sock) => sock.address.bind(addr, handle, task),
			Listening(_) => Err(Errno::EINVAL),
		}
	}

	fn listen(&self, backlog: usize) -> Result<(), Errno> {
		let mut socket = self.socket.lock_check_signal()?;

		use StreamSocket::*;
		match *socket {
			Created(ref x) => match x.clone_address() {
				Some(addr) => {
					*socket = Listening(ListeningSocket::new(addr, backlog));
					return Ok(());
				}
				None => Err(Errno::EINVAL),
			},
			_ => Err(Errno::EINVAL),
		}
	}

	fn accept(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Result<VfsSocketHandle, Errno> {
		loop {
			let socket = self.socket.lock_check_signal()?;

			use StreamSocket::*;
			let result = match *socket {
				Listening(ref x) => x.try_accept(addr),
				Created(_) | Connected(_) => return Err(Errno::EINVAL),
			};

			if let Some(x) = result {
				return x;
			}

			drop(socket);
			yield_now();
		}
	}

	fn connect(
		self: &Arc<Self>,
		addr: &UnknownSocketAddress<ReadOnly>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		let socket = self.socket.lock_check_signal()?;

		use StreamSocket::*;
		match &*socket {
			Created(_) => Ok(()),
			Connected(_) => Err(Errno::EISCONN),
			Listening(_) => Err(Errno::EINVAL),
		}?;

		drop(socket);

		let addr: LocalSocketAddress = addr.copy_to_solid()?;
		let ent = lookup_entry_follow(&addr.path, task)?;

		let sock = match ent {
			VfsRealEntry::ArcVfsSocketEntry(sock) => sock,
			_ => return Err(Errno::ECONNREFUSED),
		};

		let stream_sock = match sock.get_socket()?.expose_socket() {
			SocketHandle::LocalStream(x) => x,
			_ => return Err(Errno::ECONNREFUSED),
		};

		stream_sock.push_recv_buffer(self)?;

		loop {
			let socket = self.socket.lock_check_signal()?;

			if let Connected(_) = &*socket {
				break;
			}
		}

		Ok(())
	}
}

struct ConnectedSocket {
	address: BindAddress,
	peer: Weak<ConnectedSocket>,
	recv_buffer: LocalLocked<WrapQueue<u8, 16384>>,
}

impl ConnectedSocket {
	fn new(address: BindAddress) -> (Arc<Self>, Arc<Self>) {
		let mut socket1 = Arc::new(Self {
			address,
			peer: Weak::default(),
			recv_buffer: LocalLocked::new(WrapQueue::new()),
		});
		let s1_weak = Arc::downgrade(&socket1);

		let mut socket2 = Arc::new(Self {
			address: BindAddress::new(),
			peer: Weak::default(),
			recv_buffer: LocalLocked::new(WrapQueue::new()),
		});
		let s2_weak = Arc::downgrade(&socket2);

		unsafe {
			Arc::get_mut_unchecked(&mut socket1).peer = s2_weak;
			Arc::get_mut_unchecked(&mut socket2).peer = s1_weak;
		};

		(socket1, socket2)
	}

	fn send_to(
		&self,
		addr: &Option<UnknownSocketAddress<ReadOnly>>,
		buf: &[u8],
		_io_flag: IOFlag,
	) -> Result<usize, Errno> {
		if addr.is_some() {
			return Err(Errno::EISCONN);
		}

		let peer = match self.peer.upgrade() {
			None => return Ok(0),
			Some(peer) => peer,
		};

		let mut in_buf = buf;
		let mut total_send = 0;
		while in_buf.len() != 0 {
			let mut peer_buf = peer.recv_buffer.lock();

			let curr_send = peer_buf.write(in_buf);
			total_send += curr_send;

			let (_, remain) = in_buf.split_at(curr_send);
			in_buf = remain;
		}

		Ok(total_send)
	}

	fn recv_from(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		buf: &mut [u8],
		_io_flag: IOFlag,
	) -> Result<usize, Errno> {
		if addr.is_some() {
			return Err(Errno::EISCONN);
		}

		let mut out_buf = buf;
		let mut total_recv = 0;

		while out_buf.len() != 0 {
			let mut sock_buf = self.recv_buffer.lock();

			let curr_recv = sock_buf.read(out_buf);
			total_recv += curr_recv;

			let (_, remain) = out_buf.split_at_mut(curr_recv);
			out_buf = remain;
		}

		Ok(total_recv)
	}
}

struct ListeningSocket {
	address: LocalSocketAddress,
	backlog: usize,
	accept_buffer: LocalLocked<LinkedList<Weak<LocalStreamSocket>>>,
}

impl ListeningSocket {
	fn new(address: LocalSocketAddress, backlog: usize) -> Self {
		Self {
			address,
			backlog,
			accept_buffer: LocalLocked::default(),
		}
	}

	fn try_accept(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Option<Result<VfsSocketHandle, Errno>> {
		let client = self.accept_buffer.lock().pop_back()?.upgrade()?;

		let new_sock = match client.make_connection(addr) {
			Ok(x) => x,
			Err(e) => return Some(Err(e)),
		};

		let handle = SocketHandle::LocalStream(Arc::new(LocalStreamSocket::new(
			StreamSocket::Connected(new_sock),
		)));

		Some(Ok(VfsSocketHandle::new(
			None,
			handle,
			IOFlag::empty(),
			AccessFlag::O_RDWR,
		)))
	}
}
