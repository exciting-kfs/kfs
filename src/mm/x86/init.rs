use super::util::invalidate_all_tlb;
use super::x86_page::{PageFlag, PD, PDE};

use crate::boot::PMemory;
use crate::mm::constant::*;
use crate::mm::util::to_virt_64;

use core::mem::MaybeUninit;
use core::ops::Range;

extern "C" {
	static mut GLOBAL_PD: PD;
}

pub struct VMemory {
	pub normal: Range<u64>,
	pub high: Range<u64>,
	pub reserved: Range<u64>,
}

pub static mut VMEMORY: MaybeUninit<VMemory> = MaybeUninit::uninit();

impl VMemory {
	pub unsafe fn init(pmem: &PMemory) {
		let p_idx_max = (pmem.linear.end / PT_COVER_SIZE as u64) as usize;
		let v_idx_start = VM_OFFSET / PT_COVER_SIZE;

		let mut p_idx = 0;
		for idx in 0..PD_ENTRIES {
			if v_idx_start <= idx && p_idx < p_idx_max {
				GLOBAL_PD[idx] = PDE::new_4m(
					p_idx * PT_COVER_SIZE,
					PageFlag::Present | PageFlag::Write | PageFlag::Global,
				);
				p_idx += 1;
			} else {
				GLOBAL_PD[idx] = PDE::new_4m(0, PageFlag::empty());
			}
		}

		invalidate_all_tlb();

		let normal_start = to_virt_64(VM_OFFSET as u64);
		let normal_end = to_virt_64(p_idx as u64 * PT_COVER_SIZE as u64);

		let high_start = to_virt_64(p_idx as u64 * PT_COVER_SIZE as u64);
		let high_end = to_virt_64(pmem.linear.end);

		VMEMORY.write(VMemory {
			normal: normal_start..normal_end,
			high: high_start..high_end,
			reserved: normal_start..to_virt_64(pmem.kernel_end),
		});
	}
}
