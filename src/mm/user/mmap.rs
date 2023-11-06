use crate::{
	mm::{constant::PAGE_SIZE, util::size_to_pages},
	process::{fd_table::Fd, task::CURRENT},
	syscall::errno::Errno,
};
use bitflags::bitflags;

use super::vma::AreaFlag;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
	pub struct MmapFlag: u32 {
		const Shared = 1;
		const Private = 2;
	}
}

pub fn sys_mmap(
	addr: usize,
	len: usize,
	_prot: i32,
	flags: i32,
	fd: i32,
	offset: isize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };
	let user_ext = current.get_user_ext().expect("must be user process");

	// let flags = MmapFlag::from_bits(flags as u32).ok_or(Errno::EINVAL)?;
	// let prot = AreaFlag::from_bits(prot as u32).ok_or(Errno::EINVAL)?;

	let flags = MmapFlag::from_bits_truncate(flags as u32);
	let prot = AreaFlag::Readable | AreaFlag::Writable;

	// misaligned address
	if addr % PAGE_SIZE != 0 || len == 0 {
		return Err(Errno::EINVAL);
	}

	if flags.contains(MmapFlag::Shared) {
		let fd = Fd::from(fd as usize).ok_or(Errno::EINVAL)?;
		let handle = user_ext.lock_fd_table().get_file(fd).ok_or(Errno::EINVAL)?;
		let prot = prot.union(AreaFlag::Shared);

		user_ext
			.lock_memory()
			.mmap_shared(addr, len, handle.deep_copy()?, offset, prot)
	} else {
		let pages = size_to_pages(len);
		user_ext.lock_memory().mmap_private(addr, pages, prot)
	}
}

pub fn sys_munmap(addr: usize, len: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	if addr % PAGE_SIZE != 0 || len == 0 || len % PAGE_SIZE != 0 {
		return Err(Errno::EINVAL);
	}

	let mut memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	memory.munmap(addr, len / PAGE_SIZE).map(|_| 0)
}
