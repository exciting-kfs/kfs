use alloc::string::ToString;
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::fs::path::{format_path, Path};
use crate::fs::procfs::{PROCFS_ROOT_DIR, PROCFS_ROOT_DIR_ENTRY};
use crate::fs::tmpfs::{TmpDir, TmpSymLink};
use crate::fs::vfs::{
	lookup_entry_at_follow, DirHandle, DirInode, FileInode, Inode, Permission, Statx, StatxMode,
	StatxTimeStamp, SymLinkInode, VfsEntry, VfsHandle, VfsInode,
};
use crate::process::{fd_table::Fd, get_idle_task, relation::Pid, task::Task};
use crate::{sync::Locked, syscall::errno::Errno};

pub fn create_fd_node(pid: Pid, fd: Fd, handle: VfsHandle) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let fd_dir = procfs.get_task_inode(&pid).ok_or(Errno::ENOENT)?;

	let idx = fd.index();
	let symlink = TmpSymLink::new(
		handle
			.get_abs_path()
			.unwrap_or_else(|_| Path::new(b"/[unknown]")),
	);

	fd_dir.lock().fds.lock().insert_fd(idx, Arc::new(symlink));

	Ok(())
}

pub fn delete_fd_node(pid: Pid, fd: Fd) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };
	let dir = procfs.get_task_inode(&pid).ok_or(Errno::ENOENT)?;

	dir.lock().fds.lock().remove_fd(fd.index());

	if let Some(ent) = &*PROCFS_ROOT_DIR_ENTRY.lock() {
		let ent = lookup_entry_at_follow(
			ent.clone(),
			&format_path!("{}/fd", pid.as_raw()),
			&get_idle_task(),
		)
		.and_then(|ent| ent.downcast_dir())?;

		ent.remove_child_force(fd.index().to_string().as_bytes());
	}

	Ok(())
}

pub struct ProcFdDirInode {
	task: Arc<Task>,
	sub_files: BTreeMap<usize, Arc<TmpSymLink>>,
}

impl ProcFdDirInode {
	pub fn new(task: &Arc<Task>) -> Self {
		let mut inode = Self {
			task: task.clone(),
			sub_files: BTreeMap::new(),
		};

		if let Some(ext) = task.get_user_ext() {
			for (fd, handle) in ext.lock_fd_table().iter_opened() {
				let path = match handle.get_abs_path() {
					Ok(path) => path,
					Err(_) => Path::new(b"/[unknown]"),
				};
				inode.sub_files.insert(fd, Arc::new(TmpSymLink::new(path)));
			}
		}

		inode
	}

	pub fn insert_fd(&mut self, fd: usize, file: Arc<TmpSymLink>) {
		self.sub_files.insert(fd, file);
	}

	pub fn remove_fd(&mut self, fd: usize) {
		self.sub_files.remove(&fd);
	}
}

impl Inode for Locked<ProcFdDirInode> {
	fn stat(&self) -> Result<Statx, Errno> {
		let this = self.lock();

		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: this.task.get_uid(),
			gid: this.task.get_gid(),
			mode: StatxMode::new(StatxMode::DIRECTORY, 0o500),
			pad1: 0,
			ino: 0,
			size: 0,
			blocks: 0,
			attributes_mask: 0,
			atime: StatxTimeStamp::default(),
			btime: StatxTimeStamp::default(),
			ctime: StatxTimeStamp::default(),
			mtime: StatxTimeStamp::default(),
			rdev_major: 0,
			rdev_minor: 0,
			dev_major: 0,
			dev_minor: 0,
		})
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

impl DirInode for Locked<ProcFdDirInode> {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let this = self.lock();

		let mut v: Vec<(u8, Vec<u8>)> = this
			.sub_files
			.keys()
			.map(|x| (7, x.to_string().into()))
			.collect();

		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		let name_str = core::str::from_utf8(name).map_err(|_| Errno::ENOENT)?;
		let idx: usize = name_str.parse().map_err(|_| Errno::ENOENT)?;

		let this = self.lock();

		this.sub_files
			.get(&idx)
			.map(|x| VfsInode::SymLink(x.clone()))
			.ok_or(Errno::ENOENT)
	}

	fn mkdir(&self, _name: &[u8], _perm: Permission) -> Result<Arc<dyn DirInode>, Errno> {
		Err(Errno::EPERM)
	}

	fn rmdir(&self, _name: &[u8]) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn create(&self, _name: &[u8], _perm: Permission) -> Result<Arc<dyn FileInode>, Errno> {
		Err(Errno::EPERM)
	}

	fn unlink(&self, _name: &[u8]) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn symlink(&self, _target: &[u8], _name: &[u8]) -> Result<Arc<dyn SymLinkInode>, Errno> {
		Err(Errno::EPERM)
	}

	fn link(&self, _target: &VfsEntry, _link_name: &[u8]) -> Result<VfsInode, Errno> {
		Err(Errno::EPERM)
	}

	fn overwrite(&self, _src: &VfsEntry, _link_name: &[u8]) -> Result<VfsInode, Errno> {
		Err(Errno::EPERM)
	}
}
