use core::{mem, ptr::addr_of_mut};

use crate::{mm::user::verify::verify_ptr_mut, process::task::CURRENT};

use super::errno::Errno;

#[repr(C)]
struct UnameBuf {
	pub sysname: [u8; 65],
	pub nodename: [u8; 65],
	pub release: [u8; 65],
	pub version: [u8; 65],
	pub machine: [u8; 65],
	pub domainname: [u8; 65],
}

macro_rules! write_field {
	($dst:expr, $content:literal) => {
		addr_of_mut!($dst)
			.cast::<u8>()
			.copy_from_nonoverlapping($content.as_ptr(), $content.len())
	};
}

pub fn sys_uname(uname_buf: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let buf = verify_ptr_mut::<UnameBuf>(uname_buf, current)?;

	let mut src: UnameBuf = unsafe { mem::zeroed() };

	unsafe {
		write_field!(src.sysname, b"KFS\0");
		write_field!(src.nodename, b"kfs\0");
		write_field!(src.release, b"v0.0.9\0");
		write_field!(src.version, b"default\0");
		write_field!(src.machine, b"x86\0");
		write_field!(src.domainname, b"(none)\0");
	}

	*buf = src;

	Ok(0)
}
