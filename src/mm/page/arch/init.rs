use alloc::vec::Vec;

use super::directory::GLOBAL_PD_VIRT;
use super::{util::invalidate_all_tlb, PageFlag, PDE};
use super::{CURRENT_PD, PD};

use crate::boot;
use crate::mm::{constant::*, util::*};
use crate::sync::singleton::Singleton;

use core::ops::Range;

#[derive(Debug, Clone)]
pub struct VMemory {
	pub normal_pfn: Range<usize>,
	pub vmalloc_pfn: Range<usize>,
	pub high_pfn: Range<usize>,
	pub local_apic_pfn: usize,
	pub io_apic_pfn: Vec<usize>,
}

const ZONE_NORMAL_START: usize = VM_OFFSET / PT_COVER_SIZE;
const VMALLOC_START: usize = PD_ENTRIES - (128 * MB / PT_COVER_SIZE);

const ZONE_NORMAL_MAX_PAGES: usize = (VMALLOC_START - ZONE_NORMAL_START) * PT_COVER_SIZE;
const VMALLOC_MAX_PAGES: usize = (PT_ENTRIES - VMALLOC_START) * PT_ENTRIES;

pub(super) static VMEMORY: Singleton<VMemory> = Singleton::uninit();

pub fn get_vmemory_map() -> VMemory {
	VMEMORY.lock().clone()
}

pub unsafe fn init() {
	let pmem = boot::get_pmem_bound();
	let mut remain_pages = addr_to_pfn(pmem.end as usize);

	// normal
	let normal_pages = mapping_zone_normal(pmem.end as usize);
	let normal_start = addr_to_pfn(phys_to_virt(pmem.start as usize));
	let normal_end = addr_to_pfn(VMALLOC_OFFSET);
	remain_pages -= normal_pages;

	// vmalloc
	let vmalloc_pages = remain_pages.min(VMALLOC_MAX_PAGES);
	let vmalloc_start = addr_to_pfn(VMALLOC_OFFSET);
	let vmalloc_end = vmalloc_start + vmalloc_pages;
	remain_pages -= vmalloc_pages;

	// high
	let high_start = 1;
	let high_end = high_start + remain_pages - 1;

	VMEMORY.write(VMemory {
		normal_pfn: normal_start..normal_end,
		vmalloc_pfn: vmalloc_start..vmalloc_end,
		high_pfn: high_start..high_end,
		local_apic_pfn: 0,
		io_apic_pfn: Vec::new(),
	});

	CURRENT_PD.write(PD::new(&mut GLOBAL_PD_VIRT));
}

unsafe fn mapping_zone_normal(max_paddr: usize) -> usize {
	let mut mapped_entries = 0;
	for va_i in 0..PD_ENTRIES {
		let paddr = virt_to_phys(va_i * PT_COVER_SIZE);
		let in_normal = ZONE_NORMAL_START <= va_i && va_i < VMALLOC_START; // 0xc000_0000 ~ 0xf800_0000

		let extra_flags = if in_normal && paddr < max_paddr {
			mapped_entries += 1;
			PageFlag::Present
		} else {
			PageFlag::empty()
		};

		GLOBAL_PD_VIRT[va_i] = PDE::new_4m(paddr, PageFlag::Write | PageFlag::Global | extra_flags);
	}

	invalidate_all_tlb();
	mapped_entries * PT_ENTRIES
}
