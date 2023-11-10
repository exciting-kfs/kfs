mod mounts;
mod task;

pub use mounts::{create_mount_entry, delete_mount_entry};
pub use task::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};

use core::mem::MaybeUninit;

use alloc::string::{String, ToString};
use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::process::{get_idle_task, get_init_task, process_tree::PROCESS_TREE, relation::Pid};
use crate::sync::LocalLocked;
use crate::{sync::Locked, syscall::errno::Errno};

use task::ProcDirInode;

use self::mounts::ProcMountsInode;

use super::syscall::{FsMagic, StatFs};
use super::tmpfs::TmpDir;
use super::vfs::{
	DirHandle, DirInode, FileHandle, FileInode, FileSystem, IOFlag, Inode, MemoryFileSystem,
	Permission, Statx, StatxMode, StatxTimeStamp, SuperBlock, SymLinkInode, VfsDirEntry, VfsEntry,
	VfsInode, Whence,
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

	fn statfs(&self) -> Result<StatFs, Errno> {
		Ok(StatFs {
			kind: FsMagic::Proc,
			block_size: 4096,
			total_blocks: !0,
			free_blocks: !0,
			free_blocks_for_user: !0,
			total_inodes: !0,
			free_inodes: !0,
			id: 0,
			filename_max_length: 256,
			fregment_size: 0,
			mount_flags: 0,
			reserved: [0; 4],
		})
	}
}

static mut PROCFS_ROOT_DIR: MaybeUninit<Arc<Locked<ProcRootDirInode>>> = MaybeUninit::uninit();
static PROCFS_ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

pub struct ProcRootDirInode {
	sub_files: BTreeMap<Pid, Arc<Locked<ProcDirInode>>>,
	mounts: Arc<LocalLocked<ProcMountsInode>>,
}

impl ProcRootDirInode {
	pub fn new() -> Self {
		Self {
			sub_files: BTreeMap::new(),
			mounts: Arc::new(LocalLocked::new(ProcMountsInode::new())),
		}
	}
}

impl Locked<ProcRootDirInode> {
	pub fn insert_task_inode(&self, pid: Pid, inode: Arc<Locked<ProcDirInode>>) {
		let mut this = self.lock();

		this.sub_files.insert(pid, inode);
	}

	pub fn remove_task_inode(&self, pid: &Pid) {
		let mut this = self.lock();

		this.sub_files.remove(&pid);
	}

	pub fn get_task_inode(&self, pid: &Pid) -> Option<Arc<Locked<ProcDirInode>>> {
		let this = self.lock();

		this.sub_files.get(pid).cloned()
	}

	pub fn get_mounts(&self) -> Arc<LocalLocked<ProcMountsInode>> {
		self.lock().mounts.clone()
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
			.chain(Some((1, String::from("mounts").into())))
			.collect();

		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		if name == b"mounts" {
			return Ok(VfsInode::File(self.get_mounts()));
		}
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

pub struct ProcFileHandle {
	data: Vec<u8>,
	cursor: usize,
}

impl ProcFileHandle {
	pub fn new(data: Vec<u8>) -> Self {
		Self { data, cursor: 0 }
	}
}

impl FileHandle for LocalLocked<ProcFileHandle> {
	fn read(&self, buf: &mut [u8], _flags: IOFlag) -> Result<usize, Errno> {
		let mut this = self.lock();

		if this.data.len() <= this.cursor {
			return Ok(0);
		}

		let source = &this.data[this.cursor..];
		let size = source.len().min(buf.len());

		buf[..size].copy_from_slice(&source[..size]);

		this.cursor += size;

		Ok(size)
	}

	fn write(&self, _buf: &[u8], _flags: IOFlag) -> Result<usize, Errno> {
		Err(Errno::EBADF)
	}

	fn lseek(&self, _offset: isize, _whence: Whence) -> Result<usize, Errno> {
		Err(Errno::EBADF)
	}
}
