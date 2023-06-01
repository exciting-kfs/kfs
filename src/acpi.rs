use core::{mem::size_of, ptr::NonNull};

use acpi::{AcpiHandler, AcpiTables};

use crate::{
	mm::{
		constant::{PAGE_MASK, PAGE_SIZE},
		page::{map_mmio, unmap_mmio, PageFlag},
		util::{phys_to_virt, virt_to_phys},
	},
	pr_warn, print_stacktrace,
};

pub static mut RSDT_PADDR: usize = 0;

#[derive(Clone)]
struct AcpiH;

impl AcpiHandler for AcpiH {
	unsafe fn map_physical_region<T>(
		&self,
		physical_address: usize,
		size: usize,
	) -> acpi::PhysicalMapping<Self, T> {
		let vaddr = phys_to_virt(physical_address);
		let virtual_address = NonNull::new_unchecked(vaddr as *mut T);

		let vaddr_end = vaddr + size;
		let remain = (vaddr_end % PAGE_SIZE > 0) as usize;

		let map_start = vaddr & PAGE_MASK;
		let map_end = (vaddr_end & PAGE_MASK) + PAGE_SIZE * remain;
		let mapped_length = map_end - map_start;

		let flags = PageFlag::Global | PageFlag::Write | PageFlag::Present;

		for vaddr in (map_start..map_end).step_by(PAGE_SIZE) {
			let paddr = virt_to_phys(vaddr);
			pr_warn!("map: p: {:x}, v: {:x}", paddr, vaddr);
			map_mmio(vaddr, paddr, flags).expect("mapping_apic_table"); // FIXME ?
		}

		print_stacktrace!();

		acpi::PhysicalMapping::new(
			physical_address,
			virtual_address,
			size_of::<T>(),
			mapped_length,
			self.clone(),
		)
	}

	fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
		let vaddr = phys_to_virt(region.physical_start());

		let map_start = vaddr & PAGE_MASK;
		let map_end = map_start + region.mapped_length();

		for vaddr in (map_start..map_end).step_by(PAGE_SIZE) {
			pr_warn!("unmap: v: {:x}", vaddr);
			unmap_mmio(vaddr).expect("unmapping_apic_table"); // FIXME ?
		}

		print_stacktrace!();
	}
}

pub fn init() {
	let _ = unsafe { AcpiTables::from_rsdt(AcpiH, 0, RSDT_PADDR).unwrap() };
}
