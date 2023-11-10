mod cwd;
mod fd;
mod stat;

pub use cwd::change_cwd;
pub use fd::{create_fd_node, delete_fd_node};

use alloc::{boxed::Box, format, sync::Arc, vec};

use crate::fs::path::Path;
use crate::fs::tmpfs::{TmpDir, TmpSymLink};
use crate::fs::vfs::{
	DirHandle, DirInode, Entry, FileInode, Inode, Permission, Statx, StatxMode, StatxTimeStamp,
	SymLinkInode, VfsEntry, VfsInode,
};
use crate::process::{relation::Pid, task::Task};
use crate::{sync::Locked, syscall::errno::Errno};

use fd::ProcFdDirInode;
use stat::ProcStatInode;

use super::{PROCFS_ROOT_DIR, PROCFS_ROOT_DIR_ENTRY};

pub fn create_task_node(task: &Arc<Task>) {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	procfs.insert_task_inode(
		task.get_pid(),
		Arc::new(Locked::new(ProcDirInode::new(task))),
	);
}

pub fn delete_task_node(pid: Pid) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };
	procfs.remove_task_inode(&pid);

	if let Some(ent) = &*PROCFS_ROOT_DIR_ENTRY.lock() {
		ent.remove_child_force(format!("{}", pid.as_raw()).as_bytes());
	}

	Ok(())
}

pub struct ProcDirInode {
	task: Arc<Task>,
	cwd: Arc<TmpSymLink>,
	fds: Arc<Locked<ProcFdDirInode>>,
}

impl ProcDirInode {
	pub fn new(task: &Arc<Task>) -> Self {
		let cwd = task
			.get_user_ext()
			.ok_or(Errno::ENOENT)
			.and_then(|ext| ext.lock_cwd().get_abs_path())
			.unwrap_or_else(|_| Path::new_root());

		Self {
			task: task.clone(),
			cwd: Arc::new(TmpSymLink::new(cwd)),
			fds: Arc::new(Locked::new(ProcFdDirInode::new(task))),
		}
	}
}

impl Inode for Locked<ProcDirInode> {
	fn stat(&self) -> Result<Statx, Errno> {
		let this = self.lock();

		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: this.task.get_uid(),
			gid: this.task.get_gid(),
			mode: StatxMode::new(StatxMode::DIRECTORY, 0o555),
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

impl DirInode for Locked<ProcDirInode> {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let v = vec![
			(7, b"cwd".to_vec()),
			(2, b"fd".to_vec()),
			(1, b"stat".to_vec()),
			(2, b".".to_vec()),
			(2, b"..".to_vec()),
		];

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		match name {
			b"cwd" => Ok(VfsInode::SymLink(self.lock().cwd.clone())),
			b"fd" => Ok(VfsInode::Dir(self.lock().fds.clone())),
			b"stat" => Ok(VfsInode::File(Arc::new(ProcStatInode(
				self.lock().task.clone(),
			)))),
			_ => Err(Errno::ENOENT),
		}
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

	fn link(&self, _target: VfsEntry, _link_name: &[u8]) -> Result<VfsInode, Errno> {
		Err(Errno::EPERM)
	}
}
