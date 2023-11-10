use core::mem::MaybeUninit;

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use crate::elf::kobject::LOADED_MODULES;
use crate::process::get_init_task;
use crate::sync::Locked;
use crate::syscall::errno::Errno;

use super::syscall::{FsMagic, StatFs};
use super::tmpfs::TmpDirInode;
use super::vfs::{
	self, FileSystem, Inode, StatxMode, StatxTimeStamp, SuperBlock, VfsDirEntry, VfsEntry,
};
use super::{
	tmpfs::TmpDir,
	vfs::{
		DirHandle, DirInode, FileInode, MemoryFileSystem, Permission, Statx, SymLinkInode, VfsInode,
	},
};

pub fn init() {
	unsafe { SYSFS_ROOT_DIR.write(Arc::new(Locked::new(SysRootDirInode::new()))) };
}

pub struct SysFs;

impl FileSystem for SysFs {}

impl MemoryFileSystem for SysFs {
	fn mount() -> Result<(Arc<dyn SuperBlock>, Arc<dyn DirInode>), Errno> {
		if SYSFS_ROOT_DIR_ENTRY.lock().is_some() {
			return Err(Errno::EBUSY);
		}

		Ok((Arc::new(SysSb), unsafe {
			SYSFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}

	fn finish_mount(entry: &Arc<VfsDirEntry>) {
		SYSFS_ROOT_DIR_ENTRY.lock().replace(entry.clone());
	}
}

pub struct SysSb;

impl vfs::SuperBlock for SysSb {
	fn filesystem(&self) -> Box<dyn FileSystem> {
		Box::new(SysFs)
	}

	fn unmount(&self) -> Result<(), Errno> {
		SYSFS_ROOT_DIR_ENTRY.lock().take();

		Ok(())
	}

	fn statfs(&self) -> Result<StatFs, Errno> {
		Ok(StatFs {
			kind: FsMagic::Sys,
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

static mut SYSFS_ROOT_DIR: MaybeUninit<Arc<Locked<SysRootDirInode>>> = MaybeUninit::uninit();
static SYSFS_ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

fn sync_entry(ent: &Arc<VfsDirEntry>, mod_name: &[u8]) -> Result<(), Errno> {
	let ent = ent.lookup(b"modules", &get_init_task())?;

	let modules_dir = ent.downcast_dir()?;

	modules_dir.remove_child_force(mod_name);

	Ok(())
}

pub fn remove_module_node(mod_name: &[u8]) {
	if let Some(ent) = &*SYSFS_ROOT_DIR_ENTRY.lock() {
		_ = sync_entry(ent, mod_name);
	}
}

pub struct ModuleDirInode;

impl ModuleDirInode {
	pub fn new() -> Self {
		ModuleDirInode
	}
}

impl Inode for ModuleDirInode {
	fn stat(&self) -> Result<Statx, Errno> {
		Ok(Statx {
			mask: Statx::MASK_ALL,
			blksize: 0,
			attributes: 0,
			nlink: 0,
			uid: 0,
			gid: 0,
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

impl DirInode for ModuleDirInode {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let modules = LOADED_MODULES.lock();

		let mut v: Vec<(u8, Vec<u8>)> = modules.keys().map(|x| (2, x.to_vec())).collect();

		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		let modules = LOADED_MODULES.lock();

		if !modules.contains_key(name) {
			return Err(Errno::ENOENT);
		}

		Ok(VfsInode::Dir(TmpDirInode::new_shared(
			Permission::from_bits_truncate(0o000),
			0,
			0,
		)))
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

pub struct SysRootDirInode {
	modules: VfsInode,
}

impl SysRootDirInode {
	pub fn new() -> Self {
		Self {
			modules: VfsInode::Dir(Arc::new(ModuleDirInode)),
		}
	}
}

impl Inode for Locked<SysRootDirInode> {
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

impl DirInode for Locked<SysRootDirInode> {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let mut v: Vec<(u8, Vec<u8>)> = Vec::new();

		v.push((2, b"modules".to_vec()));
		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		if name == b"modules" {
			Ok(self.lock().modules.clone())
		} else {
			Err(Errno::ENOENT)
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
