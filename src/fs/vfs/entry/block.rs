use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	fs::{
		devfs::partition::{DevPart, PartBorrow},
		vfs::RealInode,
	},
	syscall::errno::Errno,
};

use super::{Entry, Ident, RealEntry, VfsDirEntry};

pub type ArcVfsBlockEntry = Arc<VfsBlockEntry>;

pub struct VfsBlockEntry {
	name: Rc<Vec<u8>>,
	dev: Arc<DevPart>,
	parent: Weak<VfsDirEntry>,
}

impl VfsBlockEntry {
	pub fn new(name: Rc<Vec<u8>>, dev: Arc<DevPart>, parent: Weak<VfsDirEntry>) -> Self {
		Self { name, dev, parent }
	}

	pub fn get_device(&self) -> Result<PartBorrow, Errno> {
		self.dev.get()
	}
}

impl Entry for Arc<VfsBlockEntry> {
	fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}

	fn parent_weak(&self) -> Weak<VfsDirEntry> {
		self.parent.clone()
	}
}

impl RealEntry for Arc<VfsBlockEntry> {
	fn real_inode(&self) -> Arc<dyn RealInode> {
		let dev = self.dev.clone();
		dev
	}
}
