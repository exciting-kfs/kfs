use core::mem::{replace, take};
use core::{array, ops::IndexMut};

use crate::fs::vfs::VfsHandle;
use crate::syscall::errno::Errno;

pub const FDTABLE_SIZE: usize = 256;

#[derive(Debug, Clone)]
pub struct Fd(usize);

impl Fd {
	#[inline]
	pub fn index(&self) -> usize {
		self.0
	}

	pub fn from(v: usize) -> Option<Self> {
		(v < FDTABLE_SIZE).then(|| Self(v))
	}
}

pub struct FdTable([Option<VfsHandle>; FDTABLE_SIZE]);

impl FdTable {
	pub fn new() -> Self {
		Self(array::from_fn(|_| None))
	}

	pub fn clone_for_fork(&self) -> Self {
		Self(self.0.clone())
	}

	pub fn get_file(&self, fd: Fd) -> Option<VfsHandle> {
		self.0[fd.index()].clone()
	}

	pub fn alloc_fd(&mut self, file: VfsHandle) -> Option<Fd> {
		let (fd, entry) = self
			.0
			.iter_mut()
			.enumerate()
			.find(|(_, entry)| entry.is_none())?;

		*entry = Some(file);

		Some(Fd(fd))
	}

	pub fn close(&mut self, fd: Fd) -> Result<VfsHandle, Errno> {
		let entry = self.0.index_mut(fd.index());

		take(entry).ok_or(Errno::EBADF)
	}

	pub fn dup2(&mut self, src: Fd, dst: Fd) -> Result<Option<VfsHandle>, Errno> {
		let src = self.0[src.index()].clone().ok_or(Errno::EBADF)?;

		Ok(replace(&mut self.0[dst.index()], Some(src)))
	}

	pub fn dup_start(&mut self, src_fd: Fd, start: usize) -> Result<Fd, Errno> {
		let src = self.0[src_fd.index()].clone().ok_or(Errno::EBADF)?;

		let (dst_fd, entry) = self.0[start..]
			.iter_mut()
			.enumerate()
			.find(|(_, entry)| entry.is_none())
			.map(|(i, e)| (start + i, e))
			.ok_or(Errno::EMFILE)?;

		*entry = Some(src);

		Ok(Fd(dst_fd))
	}

	pub fn dup(&mut self, src: Fd) -> Result<Fd, Errno> {
		let src = self.0[src.index()].clone().ok_or(Errno::EBADF)?;

		let new_fd = self.alloc_fd(src).ok_or(Errno::EMFILE)?;

		Ok(new_fd)
	}

	pub fn clear(&mut self) {
		self.0.iter_mut().for_each(|e| *e = None);
	}

	pub fn iter_opened(&self) -> impl '_ + Iterator<Item = (usize, VfsHandle)> {
		self.0
			.iter()
			.enumerate()
			.filter_map(|(i, x)| x.clone().map(|x| (i, x)))
	}
}
