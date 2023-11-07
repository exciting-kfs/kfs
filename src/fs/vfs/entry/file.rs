use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{fs::vfs::Inode, process::task::Task, syscall::errno::Errno};

use super::{
	AccessFlag, Entry, FileInode, IOFlag, Ident, Permission, SuperBlock, VfsDirEntry, VfsFileHandle,
};

pub struct VfsFileEntry {
	name: Rc<Vec<u8>>,
	inode: Arc<dyn FileInode>,
	parent: Weak<VfsDirEntry>,
	super_block: Arc<dyn SuperBlock>,
}

impl VfsFileEntry {
	pub fn new(
		name: Rc<Vec<u8>>,
		inode: Arc<dyn FileInode>,
		parent: Weak<VfsDirEntry>,
		super_block: Arc<dyn SuperBlock>,
	) -> Self {
		Self {
			name,
			inode,
			parent,
			super_block,
		}
	}

	pub fn open(
		self: &Arc<Self>,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Result<Arc<VfsFileHandle>, Errno> {
		let inner = self.inode.open()?;
		Ok(Arc::new(VfsFileHandle::new(
			Some(self.clone()),
			inner,
			io_flags,
			access_flags,
		)))
	}

	pub fn truncate(self: &Arc<Self>, len: isize, task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_WRITE, task)?;

		self.inode.truncate(len)
	}
}

impl Entry for Arc<VfsFileEntry> {
	fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}

	fn get_inode(&self) -> &dyn Inode {
		&*self.inode
	}

	fn parent_weak(&self) -> Weak<VfsDirEntry> {
		self.parent.clone()
	}
}
