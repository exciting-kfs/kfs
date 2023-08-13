pub mod close;
pub mod read;
pub mod write;

use alloc::sync::Arc;
use bitflags::bitflags;

use crate::syscall::errno::Errno;

pub struct File {
	pub open_flag: OpenFlag,
	pub ops: Arc<dyn FileOps>,
}

impl File {
	pub fn new(ops: Arc<dyn FileOps>, open_flag: OpenFlag) -> Self {
		Self { open_flag, ops }
	}
}

pub trait FileOps {
	fn read(&self, file: &Arc<File>, buf: &mut [u8]) -> Result<usize, Errno>;
	fn write(&self, file: &Arc<File>, buf: &[u8]) -> Result<usize, Errno>;
}

bitflags! {
	pub struct OpenFlag: u32 {
		const O_RDONLY = 0x0000;
		const O_WRONLY = 0x0001;
		const O_RDWR = 0x0002;
		const O_NONBLOCK = 0x0004;
		const O_APPEND = 0x0008;
		const O_SHLOCK = 0x0010;
		const O_EXLOCK = 0x0020;
		const O_ASYNC = 0x0040;
		const O_CREAT = 0x0200;
		const O_TRUNC = 0x0400;
		const O_EXCL = 0x0800;
		const O_NOCTTY = 0x1000;
		const O_DSYNC = 0x4000;
		const O_DIRECTORY = 0x8000;
		const O_NOFOLLOW = 0x10000;
		const O_LARGEFILE = 0x20000;
		const O_DIRECT = 0x80000;
		const O_NOATIME = 0x100000;
		const O_CLOEXEC = 0x200000;
		const __O_SYNC = 0x400000;
		const O_PATH = 0x800000;
		const __O_TMPFILE = 0x1000000;
	}
}

impl OpenFlag {
	const O_SYNC: Self = Self::__O_SYNC.union(Self::O_DSYNC);
	const O_ACCMODE: Self = Self::O_WRONLY.union(Self::O_RDWR);
}
