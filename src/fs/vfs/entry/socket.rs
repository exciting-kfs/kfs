use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{fs::vfs::RealInode, syscall::errno::Errno};

use super::{real::RealEntry, Entry, Ident, SocketInode, VfsDirEntry, VfsSocketHandle};

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
	fn parent_weak(&self) -> Weak<VfsDirEntry> {
		self.parent.clone()
	}
	fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}
}

impl RealEntry for Arc<VfsSocketEntry> {
	fn real_inode(&self) -> Arc<dyn RealInode> {
		self.inode.clone()
	}
}
