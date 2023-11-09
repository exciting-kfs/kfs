pub mod partition;

mod null;
mod tty;
mod zero;

use core::mem::MaybeUninit;

use alloc::{
	boxed::Box,
	collections::{btree_map::Entry, BTreeMap},
	sync::Arc,
	vec::Vec,
};

use crate::{config::NR_CONSOLES, driver::terminal::get_tty, sync::Locked, syscall::errno::Errno};

use self::{null::DevNull, partition::PARTITIONS, tty::DevTTY, zero::DevZero};

use super::{
	tmpfs::{TmpDir, TmpSb},
	vfs::{
		DirHandle, DirInode, FileInode, FileSystem, Ident, Inode, MemoryFileSystem, Permission,
		Statx, StatxMode, StatxTimeStamp, SuperBlock, SymLinkInode, VfsDirEntry, VfsEntry,
		VfsInode,
	},
};

pub struct DevFs;

impl FileSystem for DevFs {}

impl MemoryFileSystem for DevFs {
	fn mount() -> Result<(Arc<dyn SuperBlock>, Arc<dyn DirInode>), Errno> {
		if DEVFS_ROOT_DIR_ENTRY.lock().is_some() {
			return Err(Errno::EBUSY);
		}

		Ok((Arc::new(TmpSb), unsafe {
			DEVFS_ROOT_DIR.assume_init_ref().clone()
		}))
	}

	fn finish_mount(entry: &Arc<VfsDirEntry>) {
		DEVFS_ROOT_DIR_ENTRY.lock().replace(entry.clone());
	}
}

pub static mut DEVFS_ROOT_DIR: MaybeUninit<Arc<DevDirInode>> = MaybeUninit::uninit();
static DEVFS_ROOT_DIR_ENTRY: Locked<Option<Arc<VfsDirEntry>>> = Locked::new(None);

pub fn init() {
	partition::init();

	let dev_root_dir = DevDirInode::new();

	let mut ttyname: [u8; 4] = *b"ttyx";
	for i in 0..NR_CONSOLES {
		ttyname[3] = b'1' + i as u8;
		let ident = Ident::new(&ttyname);
		dev_root_dir.devices.lock().insert(
			ident,
			VfsInode::File(Arc::new(DevTTY::new(get_tty(i).unwrap()))),
		);
	}

	let mut partname: [u8; 5] = *b"partx";
	for (i, dev) in unsafe { &PARTITIONS }
		.iter()
		.enumerate()
		.filter_map(|(i, dev)| dev.clone().map(|x| (i, x)))
	{
		partname[4] = b'1' + i as u8;
		let ident = Ident::new(&partname);

		dev_root_dir.devices.lock().insert(ident, dev);
	}

	dev_root_dir
		.devices
		.lock()
		.insert(Ident::new(b"null"), VfsInode::File(Arc::new(DevNull)));

	dev_root_dir
		.devices
		.lock()
		.insert(Ident::new(b"zero"), VfsInode::File(Arc::new(DevZero)));

	unsafe { DEVFS_ROOT_DIR.write(Arc::new(dev_root_dir)) };
}

pub struct DevDirInode {
	devices: Locked<BTreeMap<Ident, VfsInode>>,
}

impl DevDirInode {
	pub fn new() -> Self {
		Self {
			devices: Locked::new(BTreeMap::new()),
		}
	}

	pub fn register(&self, name: &[u8], device: VfsInode) -> Result<(), Errno> {
		let ident = Ident::new(name);
		match self.devices.lock().entry(ident) {
			Entry::Occupied(_) => Err(Errno::EEXIST),
			Entry::Vacant(v) => {
				v.insert(device);
				Ok(())
			}
		}
	}

	pub fn unregister(&self, name: &[u8]) {
		self.devices.lock().remove(name);

		if let Some(ent) = &*DEVFS_ROOT_DIR_ENTRY.lock() {
			ent.remove_child_force(name);
		}
	}
}

impl Inode for DevDirInode {
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

impl DirInode for DevDirInode {
	fn open(&self) -> Result<Box<dyn DirHandle>, Errno> {
		let mut v: Vec<(u8, Vec<u8>)> = self
			.devices
			.lock()
			.keys()
			.map(|x| (3, (&*x.0).clone()))
			.collect();
		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Ok(Box::new(TmpDir::new(v)))
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		self.devices.lock().get(name).cloned().ok_or(Errno::ENOENT)
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

	fn link(&self, _src: &VfsEntry, _link_name: &[u8]) -> Result<VfsInode, Errno> {
		Err(Errno::EPERM)
	}

	fn overwrite(&self, _src: &VfsEntry, _link_name: &[u8]) -> Result<VfsInode, Errno> {
		Err(Errno::EPERM)
	}
}

pub fn register_device(name: &[u8], device: VfsInode) -> Result<(), Errno> {
	unsafe { DEVFS_ROOT_DIR.assume_init_mut().register(name, device) }
}

pub fn unregister_device(name: &[u8]) {
	unsafe { DEVFS_ROOT_DIR.assume_init_mut().unregister(name) };
}
