pub mod partition;

mod null;
mod tty;
mod zero;

use core::mem::MaybeUninit;

use alloc::{boxed::Box, collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{config::NR_CONSOLES, driver::terminal::get_tty, syscall::errno::Errno};

use self::{null::DevNull, tty::DevTTY, zero::DevZero};

use super::{
	ext2,
	tmpfs::{TmpDir, TmpSb},
	vfs::{
		DirHandle, DirInode, FileInode, FileSystem, Ident, MemoryFileSystem, Permission, RawStat,
		RealInode, SuperBlock, SymLinkInode, TimeSpec, VfsInode,
	},
};

pub struct DevFs;

impl FileSystem for DevFs {}

impl MemoryFileSystem for DevFs {
	fn mount() -> Result<(Arc<dyn SuperBlock>, Arc<dyn DirInode>), Errno> {
		Ok((Arc::new(TmpSb), unsafe {
			DEVFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}
}

pub static mut DEVFS_ROOT_DIR: MaybeUninit<Arc<DevDirInode>> = MaybeUninit::uninit();

pub fn init() {
	let mut dev_root_dir = DevDirInode::new();

	let mut ttyname: [u8; 4] = *b"ttyx";
	for i in 0..NR_CONSOLES {
		ttyname[3] = b'1' + i as u8;
		let ident = Ident::new(&ttyname);
		dev_root_dir.devices.insert(
			ident,
			VfsInode::File(Arc::new(DevTTY::new(get_tty(i).unwrap()))),
		);
	}

	let mut partname: [u8; 5] = *b"partx";
	for (i, dev) in unsafe { &ext2::PARTITIONS }
		.iter()
		.enumerate()
		.filter_map(|(i, dev)| dev.clone().map(|x| (i, x)))
	{
		partname[4] = b'1' + i as u8;
		let ident = Ident::new(&partname);

		dev_root_dir.devices.insert(ident, dev);
	}

	dev_root_dir
		.devices
		.insert(Ident::new(b"null"), VfsInode::File(Arc::new(DevNull)));

	dev_root_dir
		.devices
		.insert(Ident::new(b"zero"), VfsInode::File(Arc::new(DevZero)));

	unsafe { DEVFS_ROOT_DIR.write(Arc::new(dev_root_dir)) };
}

pub struct DevDirInode {
	devices: BTreeMap<Ident, VfsInode>,
}

impl DevDirInode {
	pub fn new() -> Self {
		Self {
			devices: BTreeMap::new(),
		}
	}
}

impl RealInode for DevDirInode {
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
}

impl DirInode for DevDirInode {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let mut v: Vec<(u8, Vec<u8>)> = self.devices.keys().map(|x| (3, (&*x.0).clone())).collect();
		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		self.devices.get(name).cloned().ok_or(Errno::ENOENT)
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
