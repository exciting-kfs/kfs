use super::util::reload_cr3;
use super::x86_page::{PageFlag, PD, PDE};

use crate::boot::MemInfo;
use crate::mm::constant::*;
use crate::mm::util::phys_to_virt;

use core::ops::Range;

extern "C" {
	static mut GLOBAL_PD: PD;
}

pub struct ZoneInfo {
	pub normal: Range<usize>,
	pub high: Range<usize>,
	pub size: usize,
}

pub unsafe fn init_linear_map(mem_info: &MemInfo) -> ZoneInfo {
	let p_idx_max = mem_info.linear.end / PT_COVER_SIZE;
	let v_idx_start = VM_OFFSET / PT_COVER_SIZE;

	let mut p_idx = 0;
	for idx in 0..PD_ENTRIES {
		if v_idx_start <= idx && p_idx < p_idx_max {
			GLOBAL_PD.entries[idx] = PDE::new_4m(
				p_idx * PT_COVER_SIZE,
				PageFlag::Present | PageFlag::Write | PageFlag::Global,
			);
			p_idx += 1;
		} else {
			GLOBAL_PD.entries[idx] = PDE::new_4m(0, PageFlag::Global);
		}
	}

	reload_cr3(&GLOBAL_PD);

	let zone_high_start = phys_to_virt(p_idx * PT_COVER_SIZE);
	let zone_normal_end = phys_to_virt((p_idx - 1) * PT_COVER_SIZE);

	return ZoneInfo {
		normal: phys_to_virt(mem_info.kernel_end)..zone_normal_end,
		high: zone_high_start..phys_to_virt(mem_info.linear.end),
		size: mem_info.linear.end,
	};
}
