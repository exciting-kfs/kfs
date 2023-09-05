use crate::{process::task::CURRENT, syscall::errno::Errno};
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Gid(AtomicUsize);

impl Gid {
	pub fn as_raw(&self) -> usize {
		self.0.load(Ordering::Relaxed)
	}

	pub fn from_raw(raw: usize) -> Self {
		Gid(AtomicUsize::new(raw))
	}

	pub fn clone(&self) -> Self {
		Gid::from_raw(self.as_raw())
	}

	pub fn set(&self, new_gid: usize) {
		self.0.store(new_gid, Ordering::Relaxed)
	}
}

pub fn sys_setgid(new_gid: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	current.set_gid(new_gid).map(|_| 0)
}

pub fn sys_getgid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_gid())
}
