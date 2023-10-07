use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	fs::{path::Path, vfs::RealInode},
	process::{get_idle_task, task::Task},
	syscall::errno::Errno,
};

use super::{
	real::RealEntry, Entry, Ident, Permission, RawStat, SocketInode, VfsDirEntry, VfsSocketHandle,
};

pub type ArcVfsSocketEntry = Arc<VfsSocketEntry>;

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

	pub fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}

	pub fn stat(&self) -> Result<RawStat, Errno> {
		self.inode.stat()
	}

	pub fn access(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		self.inode.access(task.get_uid(), task.get_gid(), perm)
	}

	pub fn chmod(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		let owner = self.stat()?.uid;

		let uid = task.get_uid();
		if uid != 0 && uid != owner {
			return Err(Errno::EPERM);
		}

		self.inode.chmod(perm)
	}

	pub fn chown(&self, owner: usize, group: usize, task: &Arc<Task>) -> Result<(), Errno> {
		if task.get_uid() != 0 {
			// TODO: group check
			return Err(Errno::EPERM);
		}

		self.inode.chown(owner, group)
	}

	pub fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		let parent = self.parent.upgrade().ok_or(Errno::ENOENT)?;

		parent
			.inode
			.access(task.get_uid(), task.get_gid(), Permission::ANY_EXECUTE)?;

		Ok(parent)
	}

	pub fn get_abs_path(&self) -> Result<Path, Errno> {
		let task = &get_idle_task();

		let mut path = Path::new_root();
		path.push_component_front(self.get_name().to_vec());

		let mut curr = self.parent_dir(task)?;
		let mut next = curr.parent_dir(task)?;
		while !Arc::ptr_eq(&curr, &next) {
			path.push_component_front(curr.get_name().to_vec());
			curr = next;
			next = curr.parent_dir(task)?;
		}

		Ok(path)
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
