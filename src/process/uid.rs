use crate::{process::task::CURRENT, syscall::errno::Errno};
use core::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug)]
pub struct Uid(AtomicUsize);
impl Uid {
	pub fn as_raw(&self) -> usize {
		self.0.load(Ordering::Relaxed)
	}

	pub fn from_raw(raw: usize) -> Self {
		Uid(AtomicUsize::new(raw))
	}

	pub fn clone(&self) -> Self {
		Uid::from_raw(self.as_raw())
	}

	pub fn set(&self, new: usize) -> Result<(), Errno> {
		// only root user (uid = 0) is authorized to change uid
		match self
			.0
			.compare_exchange(0, new, Ordering::Relaxed, Ordering::Relaxed)
		{
			Ok(_) => Ok(()),
			Err(_) => Err(Errno::EPERM),
		}
	}
}

pub fn sys_setuid(new_uid: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	current.set_uid(new_uid).map(|_| 0)
}

pub fn sys_getuid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_uid())
}
