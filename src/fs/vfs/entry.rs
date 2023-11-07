mod block;
mod dir;
mod file;
mod socket;
mod symlink;

pub use dir::VfsDirEntry;
pub use file::VfsFileEntry;
pub use socket::VfsSocketEntry;
pub use symlink::VfsSymLinkEntry;

use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};
use core::borrow::Borrow;
use enum_dispatch::enum_dispatch;

use crate::{
	fs::path::Path,
	process::{get_idle_task, task::Task},
	syscall::errno::Errno,
};

use self::block::VfsBlockEntry;

use super::{
	AccessFlag, DirInode, FileInode, IOFlag, Inode, Permission, RawStat, SocketInode, SuperBlock,
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

#[enum_dispatch(Entry)]
#[derive(Clone)]
pub enum VfsEntry {
	File(Arc<VfsFileEntry>),
	Dir(Arc<VfsDirEntry>),
	Socket(Arc<VfsSocketEntry>),
	Block(Arc<VfsBlockEntry>),
	SymLink(Arc<VfsSymLinkEntry>),
}

impl VfsEntry {
	pub fn new_dir(dir: Arc<VfsDirEntry>) -> Self {
		VfsEntry::Dir(dir)
	}

	pub fn new_file(file: Arc<VfsFileEntry>) -> Self {
		VfsEntry::File(file)
	}

	pub fn new_socket(sock: Arc<VfsSocketEntry>) -> Self {
		VfsEntry::Socket(sock)
	}

	pub fn new_block(block: Arc<VfsBlockEntry>) -> Self {
		VfsEntry::Block(block)
	}

	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Dir(d) => Ok(d),
			_ => Err(Errno::ENOTDIR),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsEntry::*;
		match self {
			File(f) => Ok(f),
			Dir(_) => Err(Errno::EISDIR),
			_ => Err(Errno::ESPIPE),
		}
	}

	pub fn downcast_socket(self) -> Result<Arc<VfsSocketEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Socket(s) => Ok(s),
			_ => Err(Errno::ECONNREFUSED),
		}
	}

	pub fn downcast_block(self) -> Result<Arc<VfsBlockEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Block(b) => Ok(b),
			Dir(_) => Err(Errno::EISDIR),
			_ => Err(Errno::ESPIPE),
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
		match self {
			File(f) => Ok(VfsHandle::File(f.open(io_flags, access_flags)?)),
			Dir(d) => Ok(VfsHandle::Dir(d.open(io_flags, access_flags)?)),
			_ => Err(Errno::ENOENT),
		}
	}
}

#[enum_dispatch]
pub trait Entry {
	fn get_inode(&self) -> &dyn Inode;

	fn stat(&self) -> Result<RawStat, Errno> {
		self.get_inode().stat()
	}

	fn access(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		self.get_inode()
			.access(task.get_uid(), task.get_gid(), perm)
	}

	fn chmod(&self, perm: Permission, task: &Arc<Task>) -> Result<(), Errno> {
		let owner = self.stat()?.uid;

		let uid = task.get_uid();
		if uid != 0 && uid != owner {
			return Err(Errno::EPERM);
		}

		self.get_inode().chmod(perm)
	}

	fn chown(&self, owner: usize, group: usize, task: &Arc<Task>) -> Result<(), Errno> {
		if task.get_uid() != 0 {
			return Err(Errno::EPERM);
		}

		self.get_inode().chown(owner, group)
	}

	fn get_name(&self) -> Ident;

	fn parent_weak(&self) -> Weak<VfsDirEntry>;

	fn parent_dir(&self, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
		let parent = self.parent_weak().upgrade().ok_or(Errno::ENOENT)?;

		parent
			.inode
			.access(task.get_uid(), task.get_gid(), Permission::ANY_EXECUTE)?;

		Ok(parent)
	}

	fn get_abs_path(&self) -> Result<Path, Errno> {
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
