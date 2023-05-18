use super::util::invalidate_all_tlb;
use super::x86_page::{PageFlag, PD, PDE};

use crate::boot::PMemory;
use crate::mm::constant::*;
use crate::mm::page_allocator::util::addr_to_pfn;
use crate::sync::singleton::Singleton;

use core::ops::Range;

extern "C" {
	static mut GLOBAL_PD: PD;
}

pub struct VMemory {
	pub normal_pfn: Range<usize>,
	pub high_pfn: Range<usize>,
}

pub static VMEMORY: Singleton<VMemory> = Singleton::uninit();

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

		let border_pfn = addr_to_pfn(VM_OFFSET);

		VMEMORY.write(VMemory {
			normal_pfn: (border_pfn + addr_to_pfn(pmem.kernel_end as usize) + 1)
				..(border_pfn + p_idx * PD_ENTRIES),
			high_pfn: 1..(p_idx_max - p_idx) * PD_ENTRIES,
		});
	}
}
