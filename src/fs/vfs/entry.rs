use core::borrow::Borrow;

use alloc::{
	collections::BTreeMap,
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	fs::path::Path,
	process::{get_idle_task, task::Task},
	sync::Locked,
	syscall::errno::Errno,
};

use super::{
	AccessFlag, DirInode, FileInode, IOFlag, Permission, RawStat, SocketInode, SuperBlock,
	SymLinkInode, VfsDirHandle, VfsFileHandle, VfsHandle, VfsInode, VfsSocketHandle,
	ROOT_DIR_ENTRY,
};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct Ident(pub Rc<Vec<u8>>);

impl Ident {
	pub fn new(name: &[u8]) -> Self {
		Ident(Rc::new(name.to_vec()))
	}

	pub fn to_vec(&self) -> Vec<u8> {
		self.0.to_vec()
	}
}

impl Borrow<[u8]> for Ident {
	fn borrow(&self) -> &[u8] {
		&self.0
	}
}

#[derive(Clone)]
pub enum VfsRealEntry {
	File(Arc<VfsFileEntry>),
	Dir(Arc<VfsDirEntry>),
	Socket(Arc<VfsSocketEntry>),
}

#[derive(Clone)]
pub enum VfsEntry {
	Real(VfsRealEntry),
	SymLink(Arc<VfsSymLinkEntry>),
}

impl VfsEntry {
	pub fn unwrap_real(self) -> VfsRealEntry {
		use VfsEntry::*;
		match self {
			Real(r) => r,
			SymLink(_) => panic!("expected Real(..) but got SymLink(..)"),
		}
	}

	pub fn new_dir(dir: Arc<VfsDirEntry>) -> Self {
		VfsEntry::Real(VfsRealEntry::Dir(dir))
	}

	pub fn new_file(file: Arc<VfsFileEntry>) -> Self {
		VfsEntry::Real(VfsRealEntry::File(file))
	}

	pub fn new_socket(sock: Arc<VfsSocketEntry>) -> Self {
		VfsEntry::Real(VfsRealEntry::Socket(sock))
	}

	pub fn get_name(&self) -> Ident {
		use VfsEntry::*;
		match self {
			Real(r) => r.get_name(),
			SymLink(s) => s.get_name(),
		}
	}

	pub fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.parent_dir(task),
			SymLink(s) => s.parent_dir(task),
		}
	}

	pub fn get_abs_path(&self) -> Result<Path, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.get_abs_path(),
			SymLink(s) => s.get_abs_path(),
		}
	}

	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.downcast_dir(),
			SymLink(_) => Err(Errno::ENOTDIR),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.downcast_file(),
			SymLink(_) => Err(Errno::EISDIR),
		}
	}
}

impl VfsRealEntry {
	pub fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.parent_dir(task),
			Dir(d) => d.parent_dir(task),
			Socket(s) => s.parent_dir(task),
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

		use VfsRealEntry::*;
		match self {
			File(f) => Ok(VfsHandle::File(f.open(io_flags, access_flags)?)),
			Dir(d) => Ok(VfsHandle::Dir(d.open(io_flags, access_flags)?)),
			Socket(_) => Err(Errno::ENOENT),
		}
	}

	pub fn get_name(&self) -> Ident {
		use VfsRealEntry::*;
		match self {
			File(f) => f.get_name(),
			Dir(d) => d.get_name(),
			Socket(s) => s.get_name(),
		}
	}

	pub fn stat(&self) -> Result<RawStat, Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.stat(),
			Dir(d) => d.stat(),
			Socket(s) => s.stat(),
		}
	}

	pub fn access(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.access(perm, task),
			Dir(d) => d.access(perm, task),
			Socket(s) => s.access(perm, task),
		}
	}

	pub fn chmod(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.chmod(perm, task),
			Dir(d) => d.chmod(perm, task),
			Socket(s) => s.chmod(perm, task),
		}
	}

	pub fn chown(&self, owner: usize, group: usize, task: &Arc<Task>) -> Result<(), Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.chown(owner, group, task),
			Dir(d) => d.chown(owner, group, task),
			Socket(s) => s.chown(owner, group, task),
		}
	}

	pub fn get_abs_path(&self) -> Result<Path, Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => f.get_abs_path(),
			Dir(d) => d.get_abs_path(),
			Socket(s) => s.get_abs_path(),
		}
	}

	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(_) | Socket(_) => Err(Errno::ENOTDIR),
			Dir(d) => Ok(d),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(f) => Ok(f),
			Dir(_) => Err(Errno::EISDIR),
			Socket(_) => Err(Errno::ESPIPE),
		}
	}

	pub fn downcast_socket(self) -> Result<Arc<VfsSocketEntry>, Errno> {
		use VfsRealEntry::*;
		match self {
			File(_) => Err(Errno::ECONNREFUSED),
			Dir(_) => Err(Errno::ECONNREFUSED),
			Socket(s) => Ok(s),
		}
	}
}

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

	pub fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		let parent = self.parent.upgrade().ok_or(Errno::ENOENT)?;

		parent
			.inode
			.access(task.get_uid(), task.get_gid(), Permission::ANY_EXECUTE)?;

		Ok(parent)
	}

	pub fn get_name(&self) -> Ident {
		Ident(self.name.clone())
	}

	pub fn get_abs_path(&self) -> Result<Path, Errno> {
		let task = &get_idle_task();

		let mut path = Path::new_root();

		let name = self.get_name();
		path.push_component_front(name.to_vec());

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
	) -> Result<Arc<VfsFileHandle>, Errno> {
		let inner = self.inode.open()?;
		Ok(Arc::new(VfsFileHandle::new(
			Some(self.clone()),
			inner,
			io_flags,
			access_flags,
		)))
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
	sub_mount: Locked<BTreeMap<Ident, VfsEntry>>,
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
			sub_mount: Locked::default(),
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

	pub fn open(
		self: &Arc<Self>,
		io_flags: IOFlag,
		access_flags: AccessFlag,
	) -> Result<Arc<VfsDirHandle>, Errno> {
		let inner = self.inode.open()?;
		Ok(Arc::new(VfsDirHandle::new(
			Some(self.clone()),
			inner,
			io_flags,
			access_flags,
		)))
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

	pub fn create(
		self: &Arc<Self>,
		name: &[u8],
		perm: Permission,
		task: &Arc<Task>,
	) -> Result<Arc<VfsFileEntry>, Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let file_inode = self.inode.create(name, perm)?;

		let file_entry = self.inode_to_entry(name, VfsInode::File(file_inode));
		self.insert_child_force(file_entry.clone());

		Ok(file_entry.downcast_file().unwrap())
	}

	pub fn mkdir(
		self: &Arc<Self>,
		name: &[u8],
		perm: Permission,
		task: &Arc<Task>,
	) -> Result<Arc<VfsDirEntry>, Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let dir_inode = self.inode.mkdir(&name, perm)?;

		let dir_entry = self.inode_to_entry(name, VfsInode::Dir(dir_inode));

		self.insert_child_force(dir_entry.clone());

		Ok(dir_entry.downcast_dir().unwrap())
	}

	pub fn unlink(&self, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		self.inode.unlink(name)?;

		self.remove_child_force(name);

		Ok(())
	}

	pub fn rmdir(&self, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		self.inode.rmdir(name)?;

		self.remove_child_force(name);

		Ok(())
	}

	pub fn symlink(
		self: &Arc<Self>,
		target: &[u8],
		name: &[u8],
		task: &Arc<Task>,
	) -> Result<Arc<VfsSymLinkEntry>, Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let inode = self.inode.symlink(target, name)?;

		Ok(Arc::new(VfsSymLinkEntry::new(
			Rc::new(name.to_vec()),
			inode,
			Arc::downgrade(self),
			Arc::clone(&self.super_block),
		)))
	}

	pub fn inode_to_entry(self: &Arc<Self>, name: &[u8], inode: VfsInode) -> VfsEntry {
		match inode {
			VfsInode::Dir(inode) => VfsEntry::new_dir(Arc::new(VfsDirEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
				false,
			))),
			VfsInode::File(inode) => VfsEntry::new_file(Arc::new(VfsFileEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
			))),
			VfsInode::Socket(inode) => VfsEntry::new_socket(Arc::new(VfsSocketEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Weak::default(),
				Arc::downgrade(self),
			))),
			VfsInode::SymLink(inode) => VfsEntry::SymLink(Arc::new(VfsSymLinkEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
			))),
		}
	}

	pub fn lookup(self: &Arc<Self>, name: &[u8], task: &Arc<Task>) -> Result<VfsEntry, Errno> {
		self.inode
			.access(task.get_uid(), task.get_gid(), Permission::ANY_EXECUTE)?;

		if let Some(x) = self.sub_mount.lock().get(name) {
			return Ok(x.clone());
		}

		if let Some(x) = self.sub_tree.lock().get(name) {
			return Ok(x.clone());
		}

		let inode = self.inode.lookup(name)?;

		let entry = self.inode_to_entry(name, inode);

		self.insert_child_force(entry.clone());

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

		let mut sub_mount = parent.sub_mount.lock();
		sub_mount.remove::<[u8]>(new_dentry.get_name().borrow());
		sub_mount.insert(new_dentry.get_name(), VfsEntry::new_dir(new_dentry.clone()));
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
			sub_mount: Locked::default(),
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
		let mut sub_mount = parent.sub_mount.lock();
		// TODO unmount cleanup
		let _ = sub_mount.remove::<[u8]>(successor.get_name().borrow());
		sub_mount.insert(successor.get_name(), VfsEntry::new_dir(successor.clone()));
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

	pub fn remove_child_force(&self, name: &[u8]) {
		let mut sub_tree = self.sub_tree.lock();

		sub_tree.remove(name);
	}

	pub fn insert_child_force(self: &Arc<Self>, entry: VfsEntry) {
		self.sub_tree.lock().insert(entry.get_name(), entry.clone());
	}
}
