use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	fs::{
		devfs::partition::{DevPart, PartBorrow},
		vfs::Inode,
	},
	syscall::errno::Errno,
};

use super::{Entry, Ident, VfsDirEntry};

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

	fn get_inode(&self) -> &dyn Inode {
		&*self.dev
	}

	fn parent_weak(&self) -> Weak<VfsDirEntry> {
		self.parent.clone()
	}
}
