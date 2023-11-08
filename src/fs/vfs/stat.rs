use super::Permission;

#[repr(C)]
#[derive(Default, Debug)]
pub struct StatxTimeStamp {
	pub sec: i64,
	pub nsec: u32,
	pub pad: i32,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug)]
pub struct StatxMode(pub u16);

impl StatxMode {
	pub const TYPE_MASK: u16 = 0o170000;

	pub const SOCKET: u16 = 0o140000;
	pub const SYMLINK: u16 = 0o120000;
	pub const REGULAR: u16 = 0o100000;
	pub const BLOCKDEV: u16 = 0o060000;
	pub const DIRECTORY: u16 = 0o040000;
	pub const CHARDEV: u16 = 0o020000;
	pub const FIFO: u16 = 0o010000;

	pub fn get_perm(&self) -> u16 {
		self.0 & !Self::TYPE_MASK
	}

	pub fn get_type(&self) -> u16 {
		self.0 & Self::TYPE_MASK
	}

	pub fn new(typ: u16, perm: u16) -> Self {
		Self((typ & Self::TYPE_MASK) | (perm & !Self::TYPE_MASK))
	}
}

#[repr(C)]
#[derive(Debug)]
pub struct Statx {
	pub mask: usize,
	pub blksize: usize,
	pub attributes: u64,
	pub nlink: usize,
	pub uid: usize,
	pub gid: usize,
	pub mode: StatxMode,
	pub pad1: u16,
	pub ino: u64,
	pub size: u64,
	pub blocks: u64,
	pub attributes_mask: u64,
	pub atime: StatxTimeStamp,
	pub btime: StatxTimeStamp,
	pub ctime: StatxTimeStamp,
	pub mtime: StatxTimeStamp,
	pub rdev_major: usize,
	pub rdev_minor: usize,
	pub dev_major: usize,
	pub dev_minor: usize,
}

impl Statx {
	pub const MASK_ALL: usize = 0x00000fff;
	pub fn get_perm(&self) -> Permission {
		Permission::from_bits_truncate(self.mode.get_perm() as u32)
	}
}
