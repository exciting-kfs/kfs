use crate::{
	mm::{constant::PAGE_SIZE, util::next_align},
	process::task::CURRENT,
	syscall::errno::Errno,
};
use bitflags::bitflags;

use super::vma::AreaFlag;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
	pub struct MmapFlag: u32 {
		const Private = 2;
	}
}

pub fn sys_mmap(
	addr: usize,
	len: usize,
	prot: i32,
	flags: i32,
	_fd: i32,
	_offset: isize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };
	let user_ext = current.get_user_ext().expect("must be user process");

	let flags = MmapFlag::from_bits(flags as u32).ok_or(Errno::EINVAL)?;
	if !flags.is_all() {
		return Err(Errno::EINVAL);
	}

	let prot = AreaFlag::from_bits(prot as u32).ok_or(Errno::EINVAL)?;

	// misaligned address
	if addr % PAGE_SIZE != 0 || len == 0 {
		return Err(Errno::EINVAL);
	}

	let pages = next_align(len - 1, PAGE_SIZE) / PAGE_SIZE;

	user_ext.lock_memory().mmap_private(addr, pages, prot)
}
