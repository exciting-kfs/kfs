use alloc::sync::Arc;
use core::array;

use crate::file::File;

const FDTABLE_SIZE: usize = 256;

#[derive(Debug)]
pub struct Fd(usize);

impl Fd {
	#[inline(always)]
	pub fn index(&self) -> usize {
		self.0
	}

	pub fn from(v: usize) -> Option<Self> {
		(v < FDTABLE_SIZE).then(|| Self(v))
	}
}

pub struct FdTable([Option<Arc<File>>; FDTABLE_SIZE]);

impl FdTable {
	pub fn new() -> Self {
		Self(array::from_fn(|_| None))
	}

	pub fn clone_for_fork(&self) -> Self {
		Self(self.0.clone())
	}

	pub fn get_file(&self, fd: Fd) -> Option<Arc<File>> {
		self.0[fd.index()].clone()
	}

	pub fn alloc_fd(&mut self, file: Arc<File>) -> Option<Fd> {
		let (fd, entry) = self
			.0
			.iter_mut()
			.enumerate()
			.find(|(_, entry)| entry.is_none())?;

		*entry = Some(file);

		Some(Fd(fd))
	}

	pub fn close(&mut self, fd: Fd) {
		self.0[fd.index()] = None;
	}

	pub fn clear(&mut self) {
		self.0.iter_mut().for_each(|e| *e = None);
	}
}
