use core::mem::MaybeUninit;
use core::str;

use alloc::collections::BTreeMap;
use alloc::{boxed::Box, string::ToString, sync::Arc, vec::Vec};
use alloc::{format, vec};

use crate::process::fd_table::Fd;
use crate::process::relation::Pid;
use crate::process::{get_idle_task, get_init_task};
use crate::sync::locked::Locked;
use crate::{
	process::{process_tree::PROCESS_TREE, task::Task},
	syscall::errno::Errno,
};

use super::path::{format_path, Path};
use super::tmpfs::TmpSb;
use super::vfs::{lookup_entry_follow, TimeSpec, VfsHandle, ROOT_DIR_ENTRY};
use super::{
	tmpfs::{TmpDir, TmpSymLink},
	vfs::{
		DirHandle, DirInode, FileInode, FileSystem, Permission, RawStat, SymLinkInode, VfsInode,
	},
};

pub fn init() -> Result<(), Errno> {
	unsafe { PROCFS_ROOT_DIR.write(Arc::new(Locked::new(ProcRootDirInode::new()))) };

	let (sb, inode) = ProcFs::mount()?;
	let root = ROOT_DIR_ENTRY.lock().clone().ok_or(Errno::ENOENT)?;

	let proc = root.mkdir(
		b"proc",
		Permission::from_bits_truncate(0o666),
		&get_idle_task(),
	)?;

	proc.mount(inode, sb, &get_idle_task())?;

	create_task_node(&get_idle_task());
	create_task_node(&get_init_task());

	Ok(())
}

pub struct ProcFs;

impl FileSystem<TmpSb, Locked<ProcRootDirInode>> for ProcFs {
	fn mount() -> Result<(Arc<TmpSb>, Arc<Locked<ProcRootDirInode>>), Errno> {
		Ok((Arc::new(TmpSb), unsafe {
			PROCFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}
}

static mut PROCFS_ROOT_DIR: MaybeUninit<Arc<Locked<ProcRootDirInode>>> = MaybeUninit::uninit();

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

	let entry = lookup_entry_follow(&Path::new(b"/proc"), &get_idle_task())
		.and_then(|ent| ent.downcast_dir())?;

	entry.remove_child_force(format!("{}", pid.as_raw()).as_bytes());

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

	let entry = lookup_entry_follow(&format_path!("/proc/{}/fd", pid.as_raw()), &get_idle_task())
		.and_then(|ent| ent.downcast_dir())?;

	entry.remove_child_force(fd.index().to_string().as_bytes());

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

	let entry = lookup_entry_follow(
		&format_path!("/proc/{}", task.get_pid().as_raw()),
		&get_idle_task(),
	)
	.and_then(|ent| ent.downcast_dir())?;

	entry.remove_child_force(b"cwd");

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

impl DirInode for Locked<ProcFdDirInode> {
	fn open(&self) -> Box<dyn DirHandle> {
		let this = self.lock();

		let mut v: Vec<Vec<u8>> = this
			.sub_files
			.keys()
			.map(|x| x.to_string().into())
			.collect();

		v.push(b".".to_vec());
		v.push(b"..".to_vec());

		Box::new(TmpDir::new(v))
	}

	fn stat(&self) -> Result<RawStat, Errno> {
		let this = self.lock();
		Ok(RawStat {
			perm: 0o500,
			uid: this.task.get_uid(),
			gid: this.task.get_gid(),
			size: 0,
			file_type: 2,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
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

impl DirInode for Locked<ProcDirInode> {
	fn open(&self) -> Box<dyn DirHandle> {
		let v = vec![
			b"cwd".to_vec(),
			b"fd".to_vec(),
			b".".to_vec(),
			b"..".to_vec(),
		];

		Box::new(TmpDir::new(v))
	}

	fn stat(&self) -> Result<RawStat, Errno> {
		let this = self.lock();

		Ok(RawStat {
			perm: 0o555,
			uid: this.task.get_uid(),
			gid: this.task.get_gid(),
			size: 0,
			file_type: 2,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		match name {
			b"cwd" => Ok(VfsInode::SymLink(self.lock().cwd.clone())),
			b"fd" => Ok(VfsInode::Dir(self.lock().fds.clone())),
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

impl DirInode for Locked<ProcRootDirInode> {
	fn open(&self) -> Box<dyn DirHandle> {
		let mut v: Vec<Vec<u8>> = PROCESS_TREE
			.lock()
			.members()
			.keys()
			.map(|x| x.as_raw().to_string().into())
			.collect();

		v.push(b".".to_vec());
		v.push(b"..".to_vec());

		Box::new(TmpDir::new(v))
	}

	fn stat(&self) -> Result<RawStat, Errno> {
		Ok(RawStat {
			perm: 0o555,
			uid: 0,
			gid: 0,
			size: 0,
			file_type: 2,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, _owner: usize, _group: usize) -> Result<(), Errno> {
		Err(Errno::EPERM)
	}

	fn chmod(&self, _perm: Permission) -> Result<(), Errno> {
		Err(Errno::EPERM)
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
