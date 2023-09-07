use alloc::{boxed::Box, sync::Arc};

use crate::syscall::errno::Errno;

use super::{AccessFlag, IOFlag, VfsDirEntry, VfsFileEntry};

#[derive(Clone)]
pub enum VfsHandle {
	File(Arc<VfsFileHandle>),
	Dir(Arc<VfsDirHandle>),
}

impl VfsHandle {
	pub fn read(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.read(buf),
			Dir(_) => Err(Errno::EISDIR),
		}
	}

	pub fn write(&self, buf: &[u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.write(buf),
			Dir(_) => Err(Errno::EISDIR),
		}
	}

	pub fn getdents(&self, buf: &mut [u8]) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(_) => Err(Errno::ENOTDIR),
			Dir(d) => d.getdents(buf),
		}
	}

	pub fn lseek(&self, offset: isize, whence: Whence) -> Result<usize, Errno> {
		use VfsHandle::*;
		match self {
			File(f) => f.lseek(offset, whence),
			Dir(_) => Err(Errno::EISDIR),
		}
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
}

#[repr(C)]
pub struct KfsDirent {
	pub ino: u32,
	pub private: u32,
	pub size: u16,
	pub name: (),
}

pub trait DirHandle {
	fn getdents(&self, buf: &mut [u8], flags: IOFlag) -> Result<usize, Errno>;
}