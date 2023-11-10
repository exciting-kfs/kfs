use crate::fs::path::Path;
use crate::fs::vfs::lookup_entry_follow_except_last;
use crate::mm::user::verify::{verify_path, verify_ptr_mut};
use crate::{process::task::CURRENT, syscall::errno::Errno};

#[repr(u32)]
#[derive(Debug, Copy, Clone)]
pub enum FsMagic {
	Ext2 = 0xef53,
	Proc = 0x9fa0,
	Dev = 0x1373,
	Tmp = 0x01021994,
	Sys = 0x62656572,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct StatFs {
	pub kind: FsMagic,
	pub block_size: usize,
	pub total_blocks: u64,
	pub free_blocks: u64,
	pub free_blocks_for_user: u64,
	pub total_inodes: u64,
	pub free_inodes: u64,
	pub id: u64,
	pub filename_max_length: usize,
	pub fregment_size: usize,
	pub mount_flags: usize,
	pub reserved: [usize; 4],
}

pub fn sys_statfs64(path: usize, _buf_size: usize, stat_buf: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let stat_buf = verify_ptr_mut::<StatFs>(stat_buf, current)?;

	let entry = lookup_entry_follow_except_last(&path, current)?;

	let sb = entry.super_block().ok_or(Errno::ENOSYS)?;

	let statfs = sb.statfs()?;

	*stat_buf = statfs;

	Ok(0)
}
