mod dir;
mod file;
mod real;
mod socket;
mod symlink;

pub use dir::VfsDirEntry;
pub use file::VfsFileEntry;
pub use real::{RealEntry, VfsRealEntry};
pub use socket::VfsSocketEntry;
pub use symlink::VfsSymLinkEntry;

use core::borrow::Borrow;

use alloc::{
	rc::Rc,
	sync::{Arc, Weak},
	vec::Vec,
};
use enum_dispatch::enum_dispatch;

use crate::{
	fs::path::Path,
	process::{get_idle_task, task::Task},
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

use crate::fs::vfs::entry::symlink::ArcVfsSymlinkEntry;

#[enum_dispatch(Entry)]
#[derive(Clone)]
pub enum VfsEntry {
	Real(VfsRealEntry),
	Symlink(ArcVfsSymlinkEntry),
}

impl VfsEntry {
	pub fn unwrap_real(self) -> VfsRealEntry {
		use VfsEntry::*;
		match self {
			Real(r) => r,
			Symlink(_) => panic!("expected Real(..) but got SymLink(..)"),
		}
	}

	pub fn new_dir(dir: Arc<VfsDirEntry>) -> Self {
		VfsEntry::Real(dir.into())
	}

	pub fn new_file(file: Arc<VfsFileEntry>) -> Self {
		VfsEntry::Real(file.into())
	}

	pub fn new_socket(sock: Arc<VfsSocketEntry>) -> Self {
		VfsEntry::Real(sock.into())
	}

	pub fn downcast_dir(self) -> Result<Arc<VfsDirEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.downcast_dir(),
			Symlink(_) => Err(Errno::ENOTDIR),
		}
	}

	pub fn downcast_file(self) -> Result<Arc<VfsFileEntry>, Errno> {
		use VfsEntry::*;
		match self {
			Real(r) => r.downcast_file(),
			Symlink(_) => Err(Errno::EISDIR),
		}
	}
}

#[enum_dispatch]
pub trait Entry {
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
