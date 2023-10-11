use alloc::sync::Arc;
use core::{
	ops::Deref,
	sync::atomic::{AtomicBool, Ordering},
};

use crate::{
	driver::partition::Partition,
	fs::vfs::{Permission, RawStat, RealInode, TimeSpec},
	syscall::errno::Errno,
};

#[derive(Debug)]
pub struct DevPart {
	inuse: AtomicBool,
	part: Arc<Partition>,
}

impl DevPart {
	pub fn new(part: Arc<Partition>) -> Self {
		Self {
			inuse: AtomicBool::new(false),
			part,
		}
	}

	pub fn get(self: &Arc<Self>) -> Result<PartBorrow, Errno> {
		self.inuse
			.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
			.map(|_| PartBorrow { dev: self.clone() })
			.map_err(|_| Errno::EBUSY)
	}

	fn release(&self) {
		let _ = self
			.inuse
			.compare_exchange(true, false, Ordering::Release, Ordering::Relaxed)
			.map(|_| self.part.clear());
	}
}

impl RealInode for DevPart {
	fn stat(&self) -> Result<RawStat, Errno> {
		Ok(RawStat {
			perm: 0o666,
			uid: 0,
			gid: 0,
			size: 0,
			file_type: 1, // HMM?
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, _owner: usize, _groupp: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

#[derive(Debug)]
pub struct PartBorrow {
	dev: Arc<DevPart>,
}

impl Deref for PartBorrow {
	type Target = Arc<Partition>;
	fn deref(&self) -> &Self::Target {
		&self.dev.part
	}
}

impl Drop for PartBorrow {
	fn drop(&mut self) {
		self.dev.release()
	}
}