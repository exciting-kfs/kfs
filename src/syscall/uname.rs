use core::ptr::addr_of_mut;

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

	unsafe {
		write_field!(buf.sysname, b"KFS\0");
		write_field!(buf.nodename, b"kfs\0");
		write_field!(buf.release, b"v0.0.9\0");
		write_field!(buf.version, b"default\0");
		write_field!(buf.machine, b"x86\0");
		write_field!(buf.domainname, b"(none)\0");
	}

	Ok(0)
}
