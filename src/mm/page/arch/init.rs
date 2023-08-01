use super::directory::{GLOBAL_PD_VIRT, KMAP_PT, VMALLOC_PT};
use super::util::invalidate_all_tlb;
use super::{PageFlag, PDE};
use super::{KERNEL_PD, PT};

use crate::boot::MEM_INFO;
use crate::mm::{constant::*, util::*};

use core::ptr::NonNull;

unsafe fn init_kernel_map() {
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

unsafe fn init_high_io_map() {
	for pfn in (addr_to_pfn(HIGH_IO_OFFSET)..LAST_PFN).step_by(PT_ENTRIES) {
		GLOBAL_PD_VIRT[pfn / PT_ENTRIES] = PDE::new_4m(
			pfn_to_addr(pfn),
			PageFlag::Present | PageFlag::Global | PageFlag::Write,
		);
	}
}

unsafe fn map_vmalloc_memory() {
	for (i, pfn) in (addr_to_pfn(VMALLOC_OFFSET)..addr_to_pfn(KMAP_OFFSET))
		.step_by(PT_ENTRIES)
		.enumerate()
	{
		let pt_phys = virt_to_phys({
			let vmalloc_pt = VMALLOC_PT.lock();
			((&*vmalloc_pt) as *const PT).add(i) as usize
		});
		GLOBAL_PD_VIRT[pfn / PT_ENTRIES] = PDE::new(
			pt_phys,
			PageFlag::Global | PageFlag::Present | PageFlag::Write,
		);
	}
}

unsafe fn map_kmap_memory() {
	for (i, pfn) in (addr_to_pfn(KMAP_OFFSET)..addr_to_pfn(HIGH_IO_OFFSET))
		.step_by(PT_ENTRIES)
		.enumerate()
	{
		let pt_phys = virt_to_phys({
			let kmap_pt = KMAP_PT.lock();
			((&*kmap_pt) as *const PT).add(i) as usize
		});
		GLOBAL_PD_VIRT[pfn / PT_ENTRIES] = PDE::new(
			pt_phys,
			PageFlag::Global | PageFlag::Present | PageFlag::Write,
		);
	}
}

pub unsafe fn init_fixed_map() {
	init_kernel_map();
	init_high_io_map();
}

pub unsafe fn init_arbitrary_map() {
	map_vmalloc_memory();
	map_kmap_memory();
}

pub unsafe fn init_kernel_pd() {
	KERNEL_PD.init(NonNull::from(&mut GLOBAL_PD_VIRT));
	invalidate_all_tlb();
	KERNEL_PD.pick_up();
}
