mod task;

pub use task::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};

use core::mem::MaybeUninit;

use alloc::string::ToString;
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::process::{get_idle_task, get_init_task, process_tree::PROCESS_TREE, relation::Pid};
use crate::{sync::Locked, syscall::errno::Errno};

use task::ProcDirInode;

use super::tmpfs::TmpDir;
use super::vfs::{
	DirHandle, DirInode, FileInode, FileSystem, Inode, MemoryFileSystem, Permission, Statx,
	StatxMode, StatxTimeStamp, SuperBlock, SymLinkInode, VfsDirEntry, VfsEntry, VfsInode,
};

pub fn init() {
	unsafe { PROCFS_ROOT_DIR.write(Arc::new(Locked::new(ProcRootDirInode::new()))) };

	create_task_node(&get_idle_task());
	create_task_node(&get_init_task());
}

pub struct ProcFs;

impl FileSystem for ProcFs {}

impl MemoryFileSystem for ProcFs {
	fn mount() -> Result<(Arc<dyn SuperBlock>, Arc<dyn DirInode>), Errno> {
		if PROCFS_ROOT_DIR_ENTRY.lock().is_some() {
			return Err(Errno::EBUSY);
		}

		Ok((Arc::new(ProcSb), unsafe {
			PROCFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}

	fn finish_mount(entry: &Arc<VfsDirEntry>) {
		PROCFS_ROOT_DIR_ENTRY.lock().replace(entry.clone());
	}
}

pub struct ProcSb;

impl SuperBlock for ProcSb {
	fn filesystem(&self) -> Box<dyn FileSystem> {
		Box::new(ProcFs)
	}

	fn unmount(&self) -> Result<(), Errno> {
		PROCFS_ROOT_DIR_ENTRY.lock().take();

		Ok(())
	}
}

static mut PROCFS_ROOT_DIR: MaybeUninit<Arc<Locked<ProcRootDirInode>>> = MaybeUninit::uninit();
static PROCFS_ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

pub struct ProcRootDirInode {
	sub_files: BTreeMap<Pid, Arc<Locked<ProcDirInode>>>,
}

impl ProcRootDirInode {
	pub fn new() -> Self {
		Self {
			sub_files: BTreeMap::new(),
		}
	}
}

impl Locked<ProcRootDirInode> {
	pub fn insert_inode(&self, pid: Pid, inode: Arc<Locked<ProcDirInode>>) {
		let mut this = self.lock();

		this.sub_files.insert(pid, inode);
	}

	pub fn remove_inode(&self, pid: &Pid) {
		let mut this = self.lock();

		this.sub_files.remove(&pid);
	}

	pub fn get_inode(&self, pid: &Pid) -> Option<Arc<Locked<ProcDirInode>>> {
		let this = self.lock();

		this.sub_files.get(pid).cloned()
	}
}

impl Inode for Locked<ProcRootDirInode> {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: 0,
			gid: 0,
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

impl DirInode for Locked<ProcRootDirInode> {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let mut v: Vec<(u8, Vec<u8>)> = PROCESS_TREE
			.lock()
			.members()
			.keys()
			.map(|x| (2, x.as_raw().to_string().into()))
			.collect();

		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		let pid = core::str::from_utf8(name).map_err(|_| Errno::ESRCH)?;
		let pid: usize = pid.to_string().parse().map_err(|_| Errno::ESRCH)?;
		let pid = Pid::from_raw(pid);

		let this = self.lock();
		let inode = this.sub_files.get(&pid).ok_or(Errno::ESRCH)?;

		Ok(VfsInode::Dir(inode.clone()))
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
