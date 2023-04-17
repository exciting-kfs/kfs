pub(crate) mod free_node;
pub(crate) mod free_list;
pub(crate) mod no_alloc_list;

use core::arch::asm;

#[derive(Debug)]
pub enum Error {
	Alloc
}

pub const fn align_with_hw_cache(bytes: usize) -> usize {
	const CACHE_LINE_SIZE : usize = 64; // L1

	if bytes % CACHE_LINE_SIZE == 0 {
		bytes
	} else {
		CACHE_LINE_SIZE * (bytes / CACHE_LINE_SIZE + 1)
	}
}

pub fn bit_scan_forward(data: usize) -> usize {
	let ret;
	unsafe {
		asm!(
			"bsf {0}, {1}",
			out(reg) ret,
			in(reg) data
		);
	}
	ret
}

pub fn bit_scan_reverse(data: usize) -> usize {
	let ret;
	unsafe {
		asm!(
			"bsr {0}, {1}",
			out(reg) ret,
			in(reg) data
		);
	}
	ret
}

mod test {
	use kfs_macro::kernel_test;
	use super::*;
	
	#[kernel_test(cache_utils)]
	fn test_bsfr() {
		let ret = bit_scan_forward(0x01);
		assert_eq!(ret, 0);
		let ret = bit_scan_forward(0x100);
		assert_eq!(ret, 8);
		let ret = bit_scan_forward(0x0101);
		assert_eq!(ret, 0);

		let ret = bit_scan_reverse(0x01);
		assert_eq!(ret, 0);
		let ret = bit_scan_reverse(0x100);
		assert_eq!(ret, 8);
		let ret = bit_scan_reverse(0x0101);
		assert_eq!(ret, 8);
	}
}
