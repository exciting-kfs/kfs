use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{fs::vfs::Inode, syscall::errno::Errno};

use super::{Entry, Ident, SocketInode, VfsDirEntry, VfsSocketHandle};

pub struct VfsSocketEntry {
	name: Rc<Vec<u8>>,
	inode: Arc<SocketInode>,
	handle: Weak<VfsSocketHandle>,
	parent: Weak<VfsDirEntry>,
}

impl VfsSocketEntry {
	pub fn new(
		name: Rc<Vec<u8>>,
		inode: Arc<SocketInode>,
		handle: Weak<VfsSocketHandle>,
		parent: Weak<VfsDirEntry>,
	) -> Self {
		Self {
			name,
			inode,
			parent,
			handle,
		}
	}

	pub fn get_socket(&self) -> Result<Arc<VfsSocketHandle>, Errno> {
		self.handle.upgrade().ok_or(Errno::ECONNREFUSED)
	}
}

impl Entry for Arc<VfsSocketEntry> {
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
