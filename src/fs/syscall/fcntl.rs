use core::mem::transmute;

use crate::{
	fs::vfs::IOFlag,
	process::{
		fd_table::{self, Fd},
		task::CURRENT,
	},
	syscall::errno::Errno,
};
#[repr(u8)]
enum Cmd {
	DupFd,
	GetFd,
	SetFd,
	GetFl,
	SetFl,
}

impl Cmd {
	fn from_usize(cmd: usize) -> Result<Self, Errno> {
		match cmd {
			x @ 0..=4 => Ok(unsafe { transmute(x as u8) }),
			_ => Err(Errno::EINVAL),
		}
	}
}

pub fn sys_fcntl(fd: isize, cmd: usize, arg: usize) -> Result<usize, Errno> {
	let fd = Fd::from(fd as usize).ok_or(Errno::EBADF)?;
	let cmd = Cmd::from_usize(cmd)?;

	use Cmd::*;
	match cmd {
		DupFd => dup_fd(fd, arg),
		GetFd => Ok(0),
		SetFd => Ok(0),
		GetFl => get_fl(fd),
		SetFl => set_fl(fd, arg),
	}
}

fn dup_fd(src: Fd, start: usize) -> Result<usize, Errno> {
	if start > fd_table::FDTABLE_SIZE {
		return Err(Errno::EINVAL);
	}

	let mut fd_table = unsafe { CURRENT.get_ref() }
		.user_ext_ok_or(Errno::EINVAL)?
		.lock_fd_table();

	fd_table.dup_start(src, start).map(|fd| fd.index())
}

fn get_fl(fd: Fd) -> Result<usize, Errno> {
	let fd_table = unsafe { CURRENT.get_ref() }
		.user_ext_ok_or(Errno::EINVAL)?
		.lock_fd_table();

	let handle = fd_table.get_file(fd).ok_or(Errno::EBADF)?;

	let io_flags = handle.io_flags();
	let access_flags = handle.access_flags();

	Ok((io_flags.bits() | access_flags.bits()) as usize)
}

fn set_fl(fd: Fd, arg: usize) -> Result<usize, Errno> {
	let fd_table = unsafe { CURRENT.get_ref() }
		.user_ext_ok_or(Errno::EINVAL)?
		.lock_fd_table();

	let handle = fd_table.get_file(fd).ok_or(Errno::EBADF)?;
	let new_flags = IOFlag::from_bits_truncate(arg as i32);

	handle.set_io_flags(new_flags).map(|_| 0)
}
