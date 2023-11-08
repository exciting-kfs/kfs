use alloc::sync::Arc;
use core::{
	ops::Deref,
	sync::atomic::{AtomicBool, Ordering},
};

use crate::{
	driver::{
		ide::ide_id::NR_IDE_DEV,
		partition::{get_block_device, Partition, NR_PRIMARY},
	},
	fs::vfs::{Inode, Permission, Statx, StatxMode, StatxTimeStamp, VfsInode},
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

impl Inode for DevPart {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: 0,
			gid: 0,
			mode: StatxMode::new(StatxMode::BLOCKDEV, 0o666),
			pad1: 0,
			ino: 0,
			size: 0,
			blocks: 0,
			attributes_mask: 0,
			atime: StatxTimeStamp::default(),
			btime: StatxTimeStamp::default(),
			ctime: StatxTimeStamp::default(),
			mtime: StatxTimeStamp::default(),
			rdev_major: 0,
			rdev_minor: 0,
			dev_major: 0,
			dev_minor: 0,
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

const __PARTITION_NONE: Option<VfsInode> = None;
pub static mut PARTITIONS: [Option<VfsInode>; NR_PRIMARY * NR_IDE_DEV] =
	[__PARTITION_NONE; NR_PRIMARY * NR_IDE_DEV];

pub fn init() {
	for i in 0..(NR_PRIMARY * NR_IDE_DEV) {
		if let Some(dev) = get_block_device(i) {
			unsafe { PARTITIONS[i] = Some(VfsInode::Block(Arc::new(DevPart::new(dev)))) };
		}
	}
}
