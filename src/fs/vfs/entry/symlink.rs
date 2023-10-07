use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{ fs::path::Path, syscall::errno::Errno};

use super::{Entry, Ident, SuperBlock, SymLinkInode, VfsDirEntry};

pub type ArcVfsSymlinkEntry = Arc<VfsSymLinkEntry>;

pub struct VfsSymLinkEntry {
	name: Rc<Vec<u8>>,
	inode: Arc<dyn SymLinkInode>,
	parent: Weak<VfsDirEntry>,
	super_block: Arc<dyn SuperBlock>,
}

impl VfsSymLinkEntry {
	pub fn new(
		name: Rc<Vec<u8>>,
		inode: Arc<dyn SymLinkInode>,
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

	pub fn target(&self) -> Result<Path, Errno> {
		self.inode.target()
	}
}

impl Entry for Arc<VfsSymLinkEntry> {
	fn parent_weak(&self) -> Weak<VfsDirEntry> {
		self.parent.clone()
	}
	fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}
}
