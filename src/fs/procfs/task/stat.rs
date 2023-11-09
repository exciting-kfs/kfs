use alloc::{boxed::Box, format, string::String, sync::Arc, vec::Vec};

use crate::fs::vfs::{
	FileHandle, FileInode, IOFlag, Inode, Permission, Statx, StatxMode, StatxTimeStamp, Whence,
};
use crate::process::task::{State, Task};
use crate::{sync::LocalLocked, syscall::errno::Errno};

pub(super) struct ProcStatInode(pub Arc<Task>);

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
	fn open(&self) -> Result<Box<dyn FileHandle>, Errno> {
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
