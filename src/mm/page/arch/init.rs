use super::directory::GLOBAL_PD_VIRT;
use super::CURRENT_PD;
use super::{util::invalidate_all_tlb, PageFlag, PDE};

use crate::boot;
use crate::mm::{constant::*, util::*};
use crate::sync::singleton::Singleton;

use core::ops::Range;

#[derive(Debug, Clone)]
pub struct VMemory {
	pub normal_pfn: Range<usize>,
	pub vmalloc_pfn: Range<usize>,
	pub high_pfn: Range<usize>,
}

const ZONE_NORMAL_START: usize = VM_OFFSET / PT_COVER_SIZE;
const VMALLOC_START: usize = PD_ENTRIES - (128 * MB / PT_COVER_SIZE);

const ZONE_NORMAL_MAX_PAGES: usize = (VMALLOC_START - ZONE_NORMAL_START) * PT_COVER_SIZE;
const VMALLOC_MAX_PAGES: usize = (PT_ENTRIES - VMALLOC_START) * PT_ENTRIES;

static VMEMORY: Singleton<VMemory> = Singleton::uninit();

pub fn get_vmemory_map() -> VMemory {
	VMEMORY.lock().clone()
}

pub unsafe fn init() {
	let pmem = boot::get_pmem_bound();
	let max_paddr = pmem.end as usize;

	let mut mapped_entries = 0;
	for i in 0..PD_ENTRIES {
		let paddr = virt_to_phys(i * PT_COVER_SIZE);

		let extra_flags = if paddr < max_paddr && ZONE_NORMAL_START <= i && i < VMALLOC_START {
			mapped_entries += 1;
			PageFlag::Present
		} else {
			PageFlag::empty()
		};

		GLOBAL_PD_VIRT[i] = PDE::new_4m(paddr, PageFlag::Write | PageFlag::Global | extra_flags);
	}

	invalidate_all_tlb();

	// TODO: CLEANUP here
	let total_pages = addr_to_pfn(pmem.end as usize) - PT_ENTRIES;

	let normal_start = addr_to_pfn(phys_to_virt(pmem.start as usize));
	let normal_end = addr_to_pfn(VMALLOC_OFFSET).min(addr_to_pfn(VM_OFFSET) + total_pages);

	let mapped_pages = mapped_entries * PT_ENTRIES;
	let unmapped_pages = total_pages - mapped_pages;

	let vmalloc_pages = unmapped_pages.min(VMALLOC_MAX_PAGES);
	let vmalloc_start = addr_to_pfn(VMALLOC_OFFSET);
	let vmalloc_end = vmalloc_start + vmalloc_pages;

	let high_pages = unmapped_pages - vmalloc_pages;
	let high_start = 1;
	let high_end = high_start + high_pages - 1;

	VMEMORY.write(VMemory {
		normal_pfn: normal_start..normal_end,
		vmalloc_pfn: vmalloc_start..vmalloc_end,
		high_pfn: high_start..high_end,
	});

	CURRENT_PD.write(&mut GLOBAL_PD_VIRT);
}
