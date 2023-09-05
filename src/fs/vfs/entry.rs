use core::borrow::Borrow;

use alloc::{
	collections::BTreeMap,
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{process::task::Task, sync::locked::Locked, syscall::errno::Errno};

use super::{
	AccessFlag, DirInode, FileInode, IOFlag, Permission, RawStat, VfsDirHandle, VfsFileHandle,
	VfsHandle, VfsInode,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(pub Rc<Vec<u8>>);

impl Ident {
	pub fn new(name: &[u8]) -> Self {
		Ident(Rc::new(name.to_vec()))
	}
}

impl Borrow<[u8]> for Ident {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}

#[derive(Clone)]
pub enum VfsEntry {
	File(Arc<VfsFileEntry>),
	Dir(Arc<VfsDirEntry>),
}

impl VfsEntry {
	pub fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			File(f) => f.parent_dir(task),
			Dir(d) => d.parent_dir(task),
		}
	}

	pub fn open(
		&self,
		io_flags: IOFlag,
		access_flags: AccessFlag,
		task: &Arc<Task>,
	) -> Result<VfsHandle, Errno> {
		let read_perm = access_flags
			.read_ok()
			.then_some(Permission::ANY_READ)
			.unwrap_or(Permission::empty());

		let write_perm = access_flags
			.write_ok()
			.then_some(Permission::ANY_WRITE)
			.unwrap_or(Permission::empty());

		let perm = read_perm | write_perm;
		self.access(perm, task)?;

		use VfsEntry::*;
		let handle = match self {
			File(f) => VfsHandle::File(f.open(io_flags, access_flags)),
			Dir(d) => VfsHandle::Dir(d.open(io_flags, access_flags)),
		};

		Ok(handle)
	}

	pub fn get_name(&self) -> Ident {
		use VfsEntry::*;
		match self {
			File(f) => f.get_name(),
			Dir(d) => d.get_name(),
		}
	}

	pub fn stat(&self) -> Result<RawStat, Errno> {
		use VfsEntry::*;
		match self {
			File(f) => f.stat(),
			Dir(d) => d.stat(),
		}
	}

	pub fn access(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsEntry::*;
		match self {
			File(f) => f.access(perm, task),
			Dir(d) => d.access(perm, task),
		}
	}

	pub fn chmod(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsEntry::*;
		match self {
			File(f) => f.chmod(perm, task),
			Dir(d) => d.chmod(perm, task),
		}
	}

	pub fn chown(&self, owner: usize, group: usize, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsEntry::*;
		match self {
			File(f) => f.chown(owner, group, task),
			Dir(d) => d.chown(owner, group, task),
		}
	}

	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			File(_) => Err(Errno::ENOTDIR),
			Dir(d) => Ok(d),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsEntry::*;
		match self {
			File(f) => Ok(f),
			Dir(_) => Err(Errno::EISDIR),
		}
	}
}

pub struct VfsFileEntry {
	name: Rc<Vec<u8>>,
	inode: Arc<dyn FileInode>,
	parent: Weak<VfsDirEntry>,
}

impl VfsFileEntry {
	pub fn new(name: Rc<Vec<u8>>, inode: Arc<dyn FileInode>, parent: Weak<VfsDirEntry>) -> Self {
		Self {
			name,
			inode,
			parent,
		}
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

	pub fn open(
		self: &Arc<Self>,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Arc<VfsFileHandle> {
		let inner = self.inode.open();
		Arc::new(VfsFileHandle::new(
			Some(self.clone()),
			inner,
			io_flags,
			access_flags,
		))
	}

	pub fn truncate(&self, len: isize, task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_WRITE, task)?;

		self.inode.truncate(len)
	}
}

pub struct VfsDirEntry {
	name: Rc<Vec<u8>>,
	inode: Arc<dyn DirInode>,
	parent: Weak<VfsDirEntry>,
	sub_mount: Locked<BTreeMap<Ident, Arc<VfsDirEntry>>>,
	sub_tree: Locked<BTreeMap<Ident, VfsEntry>>,
	next_mount: Option<Arc<VfsDirEntry>>,
}

impl VfsDirEntry {
	pub fn new(name: Rc<Vec<u8>>, inode: Arc<dyn DirInode>, parent: Weak<VfsDirEntry>) -> Self {
		Self {
			name,
			inode,
			parent,
			sub_mount: Locked::default(),
			sub_tree: Locked::default(),
			next_mount: None,
		}
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

	pub fn open(self: &Arc<Self>, io_flags: IOFlag, access_flags: AccessFlag) -> Arc<VfsDirHandle> {
		let inner = self.inode.open();
		Arc::new(VfsDirHandle::new(
			Some(self.clone()),
			inner,
			io_flags,
			access_flags,
		))
	}

	pub fn create(
		self: &Arc<Self>,
		name: &[u8],
		perm: Permission,
		task: &Arc<Task>,
	) -> Result<Arc<VfsFileEntry>, Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let file_inode = self.inode.create(name, perm)?;

		let file_entry = Arc::new(VfsFileEntry::new(
			Rc::new(name.to_vec()),
			file_inode,
			Arc::downgrade(self),
		));

		let _ = self.insert_child(VfsEntry::File(file_entry.clone()));

		Ok(file_entry)
	}

	pub fn mkdir(
		self: &Arc<Self>,
		name: &[u8],
		perm: Permission,
		task: &Arc<Task>,
	) -> Result<Arc<VfsDirEntry>, Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let dir_inode = self.inode.mkdir(&name, perm)?;

		let dir_entry = Arc::new(VfsDirEntry::new(
			Rc::new(name.to_vec()),
			dir_inode,
			Arc::downgrade(self),
		));

		let _ = self.insert_child(VfsEntry::Dir(dir_entry.clone()));

		Ok(dir_entry)
	}

	pub fn unlink(&self, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		self.inode.unlink(name)?;

		let _ = self.remove_child(name);

		Ok(())
	}

	pub fn rmdir(&self, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		self.inode.rmdir(name)?;

		let _ = self.remove_child(name);

		Ok(())
	}

	pub fn lookup(self: &Arc<Self>, name: &[u8], task: &Arc<Task>) -> Result<VfsEntry, Errno> {
		self.inode
			.access(task.get_uid(), task.get_gid(), Permission::ANY_EXECUTE)?;

		// lookup mount point first
		if let Some(x) = self.sub_mount.lock().get(name) {
			return Ok(VfsEntry::Dir(x.clone()));
		}

		// then cached dir entry
		if let Some(x) = self.sub_tree.lock().get(name) {
			return Ok(x.clone());
		}

		// slow path. lookup directally
		let node = self.inode.lookup(name).map(|x| match x {
			VfsInode::Dir(inode) => VfsEntry::Dir(Arc::new(VfsDirEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
			))),
			VfsInode::File(inode) => VfsEntry::File(Arc::new(VfsFileEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
			))),
		})?;

		self.sub_tree.lock().insert(node.get_name(), node.clone());

		Ok(node)
	}

	fn remove_child(&self, name: &[u8]) -> Result<(), Errno> {
		let mut sub_tree = self.sub_tree.lock();

		sub_tree.remove(name);

		Ok(())
	}

	fn insert_child(&self, entry: VfsEntry) -> Result<(), Errno> {
		let mut sub_tree = self.sub_tree.lock();

		sub_tree.insert(entry.get_name(), entry);

		Ok(())
	}
}
