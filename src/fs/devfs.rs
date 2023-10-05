mod null;
mod tty;
mod zero;

use core::mem::MaybeUninit;

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{
	config::NR_CONSOLES, driver::terminal::get_tty, process::get_idle_task, syscall::errno::Errno,
};

use self::{null::DevNull, tty::DevTTYFile, zero::DevZero};

use super::{
	tmpfs::{TmpDir, TmpSb},
	vfs::{
		DirHandle, DirInode, FileInode, FileSystem, Ident, Permission, RawStat, SymLinkInode,
		TimeSpec, VfsInode, ROOT_DIR_ENTRY,
	},
};

pub struct DevFs;

impl FileSystem<TmpSb, DevDirInode> for DevFs {
	fn mount() -> Result<(Arc<TmpSb>, Arc<DevDirInode>), Errno> {
		Ok((Arc::new(TmpSb), unsafe {
			DEVFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}
}

static mut DEVFS_ROOT_DIR: MaybeUninit<Arc<DevDirInode>> = MaybeUninit::uninit();

pub fn init() -> Result<(), Errno> {
	let mut dev_root_dir = DevDirInode::new();

	let mut ttyname: [u8; 4] = *b"ttyx";
	for i in 0..NR_CONSOLES {
		ttyname[3] = b'0' + (i + 1) as u8;
		let ident = Ident::new(&ttyname);
		dev_root_dir
			.devices
			.insert(ident, Arc::new(DevTTYFile::new(get_tty(i).unwrap())));
	}

	dev_root_dir
		.devices
		.insert(Ident::new(b"null"), Arc::new(DevNull));

	dev_root_dir
		.devices
		.insert(Ident::new(b"zero"), Arc::new(DevZero));

	unsafe { DEVFS_ROOT_DIR.write(Arc::new(dev_root_dir)) };

	let (sb, inode) = DevFs::mount()?;
	let root = ROOT_DIR_ENTRY.lock().clone().ok_or(Errno::ESRCH)?;

	let dev = root.mkdir(
		b"dev",
		Permission::from_bits_truncate(0o666),
		&get_idle_task(),
	)?;

	dev.mount(inode, sb, &get_idle_task())?;

	Ok(())
}

pub struct DevDirInode {
	devices: BTreeMap<Ident, Arc<dyn FileInode>>,
}

impl DevDirInode {
	pub fn new() -> Self {
		Self {
			devices: BTreeMap::new(),
		}
	}
}

impl DirInode for DevDirInode {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let mut v: Vec<(u8, Vec<u8>)> = self.devices.keys().map(|x| (3, (&*x.0).clone())).collect();
		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
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
		self.devices
			.get(name)
			.cloned()
			.ok_or(Errno::ENOENT)
			.map(|x| VfsInode::File(x))
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
