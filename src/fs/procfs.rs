use core::mem::MaybeUninit;
use core::str;

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};
use alloc::{format, vec};

use crate::process::fd_table::Fd;
use crate::process::relation::Pid;
use crate::process::task::State;
use crate::process::{get_idle_task, get_init_task};
use crate::sync::{LocalLocked, Locked};
use crate::{
	process::{process_tree::PROCESS_TREE, task::Task},
	syscall::errno::Errno,
};

use super::path::{format_path, Path};
use super::vfs::{
	self, lookup_entry_at_follow, Entry, FileHandle, FileSystem, Inode, StatxMode, StatxTimeStamp,
	SuperBlock, VfsDirEntry, VfsHandle,
};
use super::{
	tmpfs::{TmpDir, TmpSymLink},
	vfs::{
		DirHandle, DirInode, FileInode, MemoryFileSystem, Permission, Statx, SymLinkInode, VfsInode,
	},
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

impl vfs::SuperBlock for ProcSb {
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

pub fn create_task_node(task: &Arc<Task>) {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	procfs.insert_inode(
		task.get_pid(),
		Arc::new(Locked::new(ProcDirInode::new(task))),
	);
}

pub fn delete_task_node(pid: Pid) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };
	procfs.remove_inode(&pid);

	if let Some(ent) = &*PROCFS_ROOT_DIR_ENTRY.lock() {
		ent.remove_child_force(format!("{}", pid.as_raw()).as_bytes());
	}

	Ok(())
}

pub fn create_fd_node(pid: Pid, fd: Fd, handle: VfsHandle) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let fd_dir = procfs.get_inode(&pid).ok_or(Errno::ENOENT)?;

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
	let dir = procfs.get_inode(&pid).ok_or(Errno::ENOENT)?;

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

pub fn change_cwd(task: &Arc<Task>) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let dir = procfs.get_inode(&task.get_pid()).ok_or(Errno::ENOENT)?;

	let cwd = task
		.get_user_ext()
		.ok_or(Errno::ENOENT)
		.and_then(|ext| ext.lock_cwd().get_abs_path())
		.unwrap_or_else(|_| Path::new_root());

	dir.lock().cwd = Arc::new(TmpSymLink::new(cwd));

	if let Some(ent) = &*PROCFS_ROOT_DIR_ENTRY.lock() {
		let ent = lookup_entry_at_follow(
			ent.clone(),
			&format_path!("{}", task.get_pid().as_raw()),
			task,
		)
		.and_then(|ent| ent.downcast_dir())?;
		ent.remove_child_force(b"cwd");
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
		let name_str = str::from_utf8(name).map_err(|_| Errno::ENOENT)?;
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
}

struct ProcStatInode(Arc<Task>);

impl Inode for ProcStatInode {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: self.0.get_uid(),
			gid: self.0.get_gid(),
			mode: StatxMode::new(StatxMode::REGULAR, 0o444),
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

impl FileInode for ProcStatInode {
	fn open(&self) -> Result<Box<dyn vfs::FileHandle>, Errno> {
		let pid = self.0.get_pid().as_raw();
		let cmd = self.0.lock_cmd();
		let cmd: &str = core::str::from_utf8(&*cmd).unwrap_or_default();
		let ppid = self.0.get_ppid().as_raw();
		let pgrp = self.0.get_pgid().as_raw();
		let sess = self.0.get_sid().as_raw();
		let zeros = core::iter::repeat("0").take(46).intersperse(" ");

		use State::*;
		let state = match &*self.0.lock_state() {
			Running => "R",
			Sleeping => "S",
			DeepSleep => "D",
			Exited => "Z",
		};

		Ok(Box::new(LocalLocked::new(ProcStatHandle {
			data: format!(
				"{pid} ({cmd}) {state} {ppid} {pgrp} {sess} {}\n",
				zeros.collect::<String>()
			)
			.into_bytes(),
			cursor: 0,
		})))
	}

	fn truncate(&self, _length: isize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}
}

struct ProcStatHandle {
	data: Vec<u8>,
	cursor: usize,
}

impl FileHandle for LocalLocked<ProcStatHandle> {
	fn read(&self, buf: &mut [u8], _flags: vfs::IOFlag) -> Result<usize, Errno> {
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

	fn write(&self, _buf: &[u8], _flags: vfs::IOFlag) -> Result<usize, Errno> {
		Err(Errno::EBADF)
	}

	fn lseek(&self, _offset: isize, _whence: vfs::Whence) -> Result<usize, Errno> {
		Err(Errno::EBADF)
	}
}

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
		let pid = str::from_utf8(name).map_err(|_| Errno::ESRCH)?;
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
}
