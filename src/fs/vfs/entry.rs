use core::borrow::Borrow;

use alloc::{
	collections::BTreeMap,
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{process::task::Task, sync::locked::Locked, syscall::errno::Errno};

use super::{
	AccessFlag, CachePolicy, DirInode, FileInode, IOFlag, Permission, RawStat, SuperBlock,
	VfsDirHandle, VfsFileHandle, VfsHandle, VfsInode, ROOT_DIR_ENTRY,
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
	sub_tree: Locked<BTreeMap<Ident, VfsEntry>>,
	next_mount: Option<Arc<VfsDirEntry>>,
	super_block: Arc<dyn SuperBlock>,
	is_mount_point: bool,
}

impl VfsDirEntry {
	pub fn new(
		name: Rc<Vec<u8>>,
		inode: Arc<dyn DirInode>,
		parent: Weak<VfsDirEntry>,
		super_block: Arc<dyn SuperBlock>,
		is_mount_point: bool,
	) -> Self {
		Self {
			name,
			inode,
			parent,
			sub_tree: Locked::default(),
			super_block,
			next_mount: None,
			is_mount_point,
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
			Arc::clone(&self.super_block),
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
			Arc::clone(&self.super_block),
			false,
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

		// lookup cached entry
		if let Some(x) = self.sub_tree.lock().get(name) {
			return Ok(x.clone());
		}

		// slow path. lookup directally
		let (cache_policy, inode) = self.inode.lookup(name)?;

		let entry = match inode {
			VfsInode::Dir(inode) => VfsEntry::Dir(Arc::new(VfsDirEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
				false,
			))),
			VfsInode::File(inode) => VfsEntry::File(Arc::new(VfsFileEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
			))),
		};

		if let CachePolicy::Always = cache_policy {
			self.sub_tree.lock().insert(entry.get_name(), entry.clone());
		}

		Ok(entry)
	}

	pub fn is_mount_point(&self) -> bool {
		self.is_mount_point
	}

	fn do_absolute_root_mount(mut self) {
		let new_dentry = Arc::new_cyclic(|parent| {
			self.parent = parent.clone();

			self
		});

		ROOT_DIR_ENTRY.lock().replace(new_dentry);
	}

	fn do_sub_mount(mut self, parent: Arc<Self>) {
		let new_dentry = Arc::new({
			self.parent = Arc::downgrade(&parent);

			self
		});

		let mut sub_tree = parent.sub_tree.lock();
		sub_tree.remove::<[u8]>(new_dentry.get_name().borrow());
		sub_tree.insert(new_dentry.get_name(), VfsEntry::Dir(new_dentry.clone()));
	}

	pub fn mount(
		self: &Arc<Self>,
		inode: Arc<dyn DirInode>,
		super_block: Arc<dyn SuperBlock>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		if !task.is_privileged() {
			return Err(Errno::EPERM);
		}

		let parent = self.parent_dir(task)?;
		let new_dentry = VfsDirEntry {
			name: Rc::clone(&self.name),
			inode,
			parent: Weak::default(),
			sub_tree: Locked::default(),
			next_mount: Some(self.clone()),
			super_block,
			is_mount_point: true,
		};

		match Arc::ptr_eq(self, &parent) {
			true => new_dentry.do_absolute_root_mount(),
			false => new_dentry.do_sub_mount(parent),
		};

		Ok(())
	}

	fn do_absolute_root_unmount(successor: Arc<Self>) {
		ROOT_DIR_ENTRY.lock().replace(successor);
	}

	fn do_sub_unmount(successor: Arc<Self>, parent: Arc<Self>) {
		let mut sub_tree = parent.sub_tree.lock();
		sub_tree.remove::<[u8]>(successor.get_name().borrow());
		sub_tree.insert(successor.get_name(), VfsEntry::Dir(successor.clone()));
	}

	pub fn unmount(self: Arc<Self>, task: &Arc<Task>) -> Result<(), Errno> {
		if !task.is_privileged() {
			return Err(Errno::EPERM);
		}

		if !self.is_mount_point() {
			return Err(Errno::EINVAL);
		}

		let parent = self.parent_dir(task)?;
		let successor = self.next_mount.clone().ok_or(Errno::EBUSY)?;

		match Arc::ptr_eq(&self, &parent) {
			true => Self::do_absolute_root_unmount(successor),
			false => Self::do_sub_unmount(successor, parent),
		};

		Ok(())
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
