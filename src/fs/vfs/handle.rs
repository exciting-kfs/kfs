use alloc::{boxed::Box, sync::Arc};

use crate::fs::path::Path;
use crate::net::address::{ReadOnly, UnknownSocketAddress, WriteOnly};
use crate::net::socket::{Socket, SocketHandle};
use crate::process::task::Task;
use crate::syscall::errno::Errno;

use super::{AccessFlag, IOFlag, VfsDirEntry, VfsEntry, VfsFileEntry, VfsSocketEntry};

#[derive(Clone)]
pub enum VfsHandle {
	File(Arc<VfsFileHandle>),
	Socket(Arc<VfsSocketHandle>),
	Dir(Arc<VfsDirHandle>),
}

impl VfsHandle {
	pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.read(buf),
			Socket(s) => s.recv_from(&mut None, buf),
			Dir(_) => Err(Errno::EISDIR),
		}
	}

	pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.write(buf),
			Socket(s) => s.send_to(&None, buf),
			Dir(_) => Err(Errno::EISDIR),
		}
	}

	pub fn close(&self) -> Result<(), Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.close(),
			Dir(d) => d.close(),
			Socket(_) => Ok(()),
		}
	}

	pub fn getdents(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(_) | Socket(_) => Err(Errno::ENOTDIR),
			Dir(d) => d.getdents(buf),
		}
	}

	pub fn lseek(&self, offset: isize, whence: Whence) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.lseek(offset, whence),
			Socket(_) => Err(Errno::ESPIPE),
			Dir(_) => Err(Errno::EISDIR),
		}
	}

	fn as_entry(&self) -> Option<VfsEntry> {
		use VfsHandle::*;
		match self {
			File(f) => f.entry.clone().map(|ent| VfsEntry::new_file(ent)),
			Socket(s) => s.entry.clone().map(|ent| VfsEntry::new_socket(ent)),
			Dir(d) => d.entry.clone().map(|ent| VfsEntry::new_dir(ent)),
		}
	}

	pub fn get_abs_path(&self) -> Result<Path, Errno> {
		let ent = self.as_entry().ok_or(Errno::EPIPE)?;

		ent.get_abs_path()
	}
}

pub struct VfsFileHandle {
	entry: Option<Arc<VfsFileEntry>>,
	inner: Box<dyn FileHandle>,
	io_flags: IOFlag,
	access_flags: AccessFlag,
}

impl VfsFileHandle {
	pub fn new(
		entry: Option<Arc<VfsFileEntry>>,
		inner: Box<dyn FileHandle>,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Self {
		Self {
			entry,
			inner,
			io_flags,
			access_flags,
		}
	}

	pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		match self.access_flags.read_ok() {
			true => self.inner.read(buf, self.io_flags),
			false => Err(Errno::EBADF),
		}
	}

	pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
		if !self.access_flags.write_ok() {
			return Err(Errno::EBADF);
		}

		if self.io_flags.contains(IOFlag::O_APPEND) {
			self.inner.lseek(0, Whence::End)?;
		}
		self.inner.write(buf, self.io_flags)
	}

	pub fn lseek(&self, offset: isize, whence: Whence) -> Result<usize, Errno> {
		self.inner.lseek(offset, whence)
	}

	pub fn close(&self) -> Result<(), Errno> {
		self.inner.close()
	}
}

pub struct VfsDirHandle {
	entry: Option<Arc<VfsDirEntry>>,
	inner: Box<dyn DirHandle>,
	io_flags: IOFlag,
	access_flags: AccessFlag,
}

impl VfsDirHandle {
	pub fn new(
		entry: Option<Arc<VfsDirEntry>>,
		inner: Box<dyn DirHandle>,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Self {
		Self {
			entry,
			inner,
			io_flags,
			access_flags,
		}
	}

	pub fn getdents(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		match self.access_flags.read_ok() {
			true => self.inner.getdents(buf, self.io_flags),
			false => Err(Errno::EBADF),
		}
	}

	pub fn close(&self) -> Result<(), Errno> {
		self.inner.close()
	}
}

pub struct VfsSocketHandle {
	entry: Option<Arc<VfsSocketEntry>>,
	inner: SocketHandle,
	io_flags: IOFlag,
	access_flags: AccessFlag,
}

macro_rules! socket_dispatch {
	($inner:expr => $method:ident($($arg:expr),*)) => {{
		use SocketHandle::*;
		match $inner {
			LocalDgram(ref x) => x.$method($($arg),*),
			LocalStream(ref x) => x.$method($($arg),*),
		}
	}};
}

impl VfsSocketHandle {
	pub fn new(
		entry: Option<Arc<VfsSocketEntry>>,
		inner: SocketHandle,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Self {
		Self {
			entry,
			inner,
			io_flags,
			access_flags,
		}
	}

	pub fn expose_socket(&self) -> SocketHandle {
		self.inner.clone()
	}

	pub fn send_to(
		&self,
		addr: &Option<UnknownSocketAddress<ReadOnly>>,
		buf: &[u8],
	) -> Result<usize, Errno> {
		if !self.access_flags.write_ok() {
			return Err(Errno::EBADF);
		}

		socket_dispatch!(self.inner => send_to(addr, buf, self.io_flags))
	}

	pub fn recv_from(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
		buf: &mut [u8],
	) -> Result<usize, Errno> {
		if !self.access_flags.read_ok() {
			return Err(Errno::EBADF);
		}

		socket_dispatch!(self.inner => recv_from(addr, buf, self.io_flags))
	}

	pub fn bind(
		self: &Arc<Self>,
		addr: &UnknownSocketAddress<ReadOnly>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		socket_dispatch!(self.inner => bind(addr, self, task))
	}

	pub fn listen(&self, backlog: usize) -> Result<(), Errno> {
		socket_dispatch!(self.inner => listen(backlog))
	}

	pub fn accept(
		&self,
		addr: &mut Option<UnknownSocketAddress<WriteOnly>>,
	) -> Result<VfsSocketHandle, Errno> {
		socket_dispatch!(self.inner => accept(addr))
	}

	pub fn connect(
		&self,
		addr: &UnknownSocketAddress<ReadOnly>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		socket_dispatch!(self.inner => connect(addr, task))
	}
}

#[derive(Clone, Copy)]
pub enum Whence {
	Begin,
	End,
	Current,
}

pub trait FileHandle {
	fn read(&self, buf: &mut [u8], flags: IOFlag) -> Result<usize, Errno>;
	fn write(&self, buf: &[u8], flags: IOFlag) -> Result<usize, Errno>;
	fn lseek(&self, offset: isize, whence: Whence) -> Result<usize, Errno>;
	fn close(&self) -> Result<(), Errno> {
		Ok(())
	}
}

#[repr(C)]
pub struct KfsDirent {
	pub ino: u32,
	pub private: u32,
	pub size: u16,
	pub file_type: u8,
	pub name: (),
}

pub trait DirHandle {
	fn getdents(&self, buf: &mut [u8], flags: IOFlag) -> Result<usize, Errno>;
	fn close(&self) -> Result<(), Errno> {
		Ok(())
	}
}
