use alloc::sync::Arc;

use crate::fs::vfs::{IOFlag, VfsSocketHandle};
use crate::{process::task::Task, syscall::errno::Errno};

use super::address::{ReadOnly, UnknownSocketAddress, WriteOnly};

#[derive(Clone)]
// TODO
pub enum SocketHandle {}

#[derive(PartialEq, Eq)]
pub enum SocketKind {
	Stream = 1,
	Dgram = 2,
}

impl SocketKind {
	pub fn from_raw(raw: i32) -> Result<Self, Errno> {
		match raw {
			1 => Ok(SocketKind::Stream),
			2 => Ok(SocketKind::Dgram),
			_ => Err(Errno::EINVAL),
		}
	}
}

pub trait Socket {
	fn send_to(
		&self,
		addr: &Option<UnknownSocketAddress<ReadOnly>>,
		buf: &[u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno>;

	fn recv_from(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		buf: &mut [u8],
		io_flag: IOFlag,
	) -> Result<usize, Errno>;

	fn bind(
		&self,
		addr: &UnknownSocketAddress<ReadOnly>,
		handle: &Arc<VfsSocketHandle>,
		task: &Arc<Task>,
	) -> Result<(), Errno>;

	fn listen(&self, bakclog: usize) -> Result<(), Errno>;
	fn accept(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Result<VfsSocketHandle, Errno>;
	fn connect(
		self: &Arc<Self>,
		addr: &UnknownSocketAddress<ReadOnly>,
		task: &Arc<Task>,
	) -> Result<(), Errno>;
}
