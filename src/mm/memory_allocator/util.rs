use core::alloc::Layout;

use crate::mm::util::bit_scan_reverse;

pub const LEVEL_MIN: usize = 6;
pub const LEVEL_END: usize = 12;
pub const LEVEL_RNG: usize = LEVEL_END - LEVEL_MIN;

pub fn level_of(layout: Layout) -> usize {
	let size = layout.size();
	let align = layout.align();

	if size <= 1 && align == 1 {
		return LEVEL_MIN;
	}

	let rank = unsafe {
		match size > align {
			true => bit_scan_reverse(size - 1) + 1,
			false => bit_scan_reverse(align - 1) + 1,
		}
	};

	LEVEL_MIN + rank.checked_sub(LEVEL_MIN).unwrap_or_default()
}

pub enum GFP {
	Atomic,
	Normal,
}
