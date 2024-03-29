use core::borrow::Borrow;

use alloc::{
	collections::BTreeMap,
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};

use crate::{
	fs::vfs::{entry::block::VfsBlockEntry, Inode},
	process::task::Task,
	sync::{LocalLocked, Locked},
	syscall::errno::Errno,
};

use super::{
	AccessFlag, DirInode, Entry, IOFlag, Ident, Permission, SuperBlock, VfsDirHandle, VfsEntry,
	VfsFileEntry, VfsInode, VfsSocketEntry, VfsSymLinkEntry, ROOT_DIR_ENTRY,
};

pub struct VfsDirEntry {
	name: Rc<Vec<u8>>,
	pub(super) inode: Arc<dyn DirInode>,
	pub(super) super_block: Arc<dyn SuperBlock>,
	parent: Weak<VfsDirEntry>,
	sub_tree: Locked<BTreeMap<Ident, VfsEntry>>,
	sub_mount: LocalLocked<BTreeMap<Ident, VfsEntry>>,
	next_mount: Option<Arc<VfsDirEntry>>,
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
			sub_mount: LocalLocked::default(),
			super_block,
			next_mount: None,
			is_mount_point,
		}
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

	pub fn unlink(self: &Arc<Self>, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		self.inode.unlink(name)?;

		self.remove_child_force(name);

		Ok(())
	}

	pub fn rmdir(self: &Arc<Self>, name: &[u8], task: &Arc<Task>) -> Result<(), Errno> {
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

	pub fn link(
		self: &Arc<Self>,
		target: &VfsEntry,
		link_name: &[u8],
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let inode = self.inode.link(target, link_name)?;
		let entry = self.inode_to_entry(link_name, inode);

		self.insert_child_force(entry);

		Ok(())
	}

	pub fn overwrite(
		self: &Arc<Self>,
		src: &VfsEntry,
		link_name: &[u8],
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		self.access(Permission::ANY_EXECUTE | Permission::ANY_WRITE, task)?;

		let inode = self.inode.overwrite(src, link_name)?;
		let entry = self.inode_to_entry(link_name, inode);

		self.insert_child_force(entry);

		Ok(())
	}

	pub fn inode_to_entry(self: &Arc<Self>, name: &[u8], inode: VfsInode) -> VfsEntry {
		use VfsInode::*;
		match inode {
			Dir(inode) => VfsEntry::new_dir(Arc::new(VfsDirEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
				false,
			))),
			File(inode) => VfsEntry::new_file(Arc::new(VfsFileEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
			))),
			Socket(inode) => VfsEntry::new_socket(Arc::new(VfsSocketEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Weak::default(),
				Arc::downgrade(self),
			))),
			SymLink(inode) => VfsEntry::SymLink(Arc::new(VfsSymLinkEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
				Arc::clone(&self.super_block),
			))),
			Block(inode) => VfsEntry::new_block(Arc::new(VfsBlockEntry::new(
				Rc::new(name.to_vec()),
				inode,
				Arc::downgrade(self),
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

	pub fn super_block(&self) -> &Arc<dyn SuperBlock> {
		&self.super_block
	}

	fn do_absolute_root_mount(mut self) -> Arc<VfsDirEntry> {
		let new_dentry = Arc::new_cyclic(|parent| {
			self.parent = parent.clone();

			self
		});

		ROOT_DIR_ENTRY.lock().replace(new_dentry.clone());

		new_dentry
	}

	fn do_sub_mount(mut self, parent: Arc<Self>) -> Arc<VfsDirEntry> {
		let new_dentry = Arc::new({
			self.parent = Arc::downgrade(&parent);

			self
		});

		let mut sub_mount = parent.sub_mount.lock();
		sub_mount.remove::<[u8]>(new_dentry.get_name().borrow());
		sub_mount.insert(new_dentry.get_name(), VfsEntry::new_dir(new_dentry.clone()));

		new_dentry
	}

	pub fn mount(
		self: &Arc<Self>,
		inode: Arc<dyn DirInode>,
		super_block: Arc<dyn SuperBlock>,
		task: &Arc<Task>,
	) -> Result<Arc<VfsDirEntry>, Errno> {
		if !task.is_privileged() {
			return Err(Errno::EPERM);
		}

		let parent = self.parent_dir(task)?;
		let new_dentry = VfsDirEntry {
			name: Rc::clone(&self.name),
			inode,
			parent: Weak::default(),
			sub_tree: Locked::default(),
			sub_mount: LocalLocked::default(),
			next_mount: Some(self.clone()),
			super_block,
			is_mount_point: true,
		};

		let new_dentry = match Arc::ptr_eq(self, &parent) {
			true => new_dentry.do_absolute_root_mount(),
			false => new_dentry.do_sub_mount(parent),
		};

		Ok(new_dentry)
	}

	fn do_absolute_root_unmount(successor: Arc<Self>) -> Result<(), Errno> {
		let mut root = ROOT_DIR_ENTRY.lock();

		if let Some(dir) = &*root {
			let fs = dir.super_block.filesystem();
			fs.unmount(&dir.super_block)?;
		}

		root.replace(successor);

		Ok(())
	}

	fn do_sub_unmount(successor: Arc<Self>, parent: Arc<Self>) -> Result<(), Errno> {
		let mut sub_mount = parent.sub_mount.lock();

		// why the type of sub_mount is Map<_, VfsEntry> rather than Map<_, VfsDirEntry>?
		if let Some(dir) = sub_mount
			.get::<[u8]>(successor.get_name().borrow())
			.and_then(|e| e.clone().downcast_dir().ok())
		{
			let fs = dir.super_block.filesystem();
			fs.unmount(&dir.super_block)?;
		}

		sub_mount.insert(successor.get_name(), VfsEntry::new_dir(successor.clone()));
		Ok(())
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
		}
	}

	pub fn remove_child_force(&self, name: &[u8]) {
		let mut sub_tree = self.sub_tree.lock();

		sub_tree.remove(name);
	}

	pub fn insert_child_force(self: &Arc<Self>, entry: VfsEntry) {
		self.sub_tree.lock().insert(entry.get_name(), entry);
	}
}

impl Entry for Arc<VfsDirEntry> {
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
