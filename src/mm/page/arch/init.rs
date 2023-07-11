use super::directory::GLOBAL_PD_VIRT;
use super::table::KMAP_PT;
use super::{util::invalidate_all_tlb, PageFlag, PDE};
use super::{KERNEL_PD, PD};

use crate::boot::MEM_INFO;
use crate::mm::{constant::*, util::*};

use core::ptr::NonNull;

pub unsafe fn map_kernel_memory() {
	for (paddr, vaddr) in (0..PD_ENTRIES)
		.map(|x| x * PT_COVER_SIZE)
		.map(|x| (x, x.wrapping_add(VM_OFFSET)))
	{
		let flags = if addr_to_pfn(paddr) < MEM_INFO.high_start_pfn {
			PageFlag::Present | PageFlag::Global | PageFlag::Write
		} else {
			PageFlag::empty() // not present
		};

		GLOBAL_PD_VIRT[vaddr / PT_COVER_SIZE] = PDE::new_4m(paddr, flags);
	}
}

unsafe fn map_high_io_memory() {
	for pfn in (addr_to_pfn(HIGH_IO_OFFSET)..LAST_PFN).step_by(PT_ENTRIES) {
		GLOBAL_PD_VIRT[pfn / PT_ENTRIES] = PDE::new_4m(
			pfn_to_addr(pfn),
			PageFlag::Present | PageFlag::Global | PageFlag::Write,
		);
	}
}

unsafe fn map_kmap_memory() {
	GLOBAL_PD_VIRT[KMAP_OFFSET / PT_COVER_SIZE] = PDE::new(
		virt_to_phys(&KMAP_PT as *const _ as usize),
		PageFlag::empty() | PageFlag::Global,
	);
}

pub unsafe fn init() {
	map_kernel_memory();
	map_high_io_memory();

	invalidate_all_tlb();

	KERNEL_PD.write(PD::new(NonNull::from(&mut GLOBAL_PD_VIRT)));
}
