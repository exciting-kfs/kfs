use core::{mem::size_of, ptr::NonNull};

use acpi::AcpiHandler;
use alloc::collections::BTreeMap;

use crate::{
	mm::{
		constant::{PAGE_MASK, PAGE_SIZE},
		page::{map_page, unmap_page, PageFlag},
		util::{phys_to_virt, virt_to_phys},
	},
	sync::singleton::Singleton,
};

static ADDRESS_REFCOUNT: Singleton<BTreeMap<usize, usize>> = Singleton::uninit();

fn acpi_map(vaddr: usize, paddr: usize, flags: PageFlag) {
	ADDRESS_REFCOUNT
		.lock()
		.entry(vaddr)
		.and_modify(|curr| *curr += 1)
		.or_insert_with(|| {
			map_page(vaddr, paddr, flags).expect("mapping_apic_table");
			1
		});
}

fn acpi_unmap(vaddr: usize) {
	let mut ref_count = ADDRESS_REFCOUNT.lock();
	let v = ref_count.remove(&vaddr);
	match v {
		None => panic!("invalid acpi unmap"),
		Some(v) => {
			if v > 1 {
				ref_count.insert(vaddr, v - 1);
			} else {
				unmap_page(vaddr).expect("unmapping_apic_table");
			}
		}
	}
}

#[derive(Clone)]
pub struct AcpiH;

impl AcpiHandler for AcpiH {
	unsafe fn map_physical_region<T>(
		&self,
		physical_address: usize,
		size: usize,
	) -> acpi::PhysicalMapping<Self, T> {
		let vaddr = phys_to_virt(physical_address);
		// pr_info!();
		let virtual_address = NonNull::new_unchecked(vaddr as *mut T);

		let vaddr_end = vaddr + size;
		let remain = (vaddr_end % PAGE_SIZE > 0) as usize;

		let map_start = vaddr & PAGE_MASK;
		let map_end = (vaddr_end & PAGE_MASK) + PAGE_SIZE * remain;
		let mapped_length = map_end - map_start;

		let flags = PageFlag::Global | PageFlag::Write | PageFlag::Present;

		for vaddr in (map_start..map_end).step_by(PAGE_SIZE) {
			let paddr = virt_to_phys(vaddr);
			acpi_map(vaddr, paddr, flags); // FIXME ?
		}

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
			acpi_unmap(vaddr); // FIXME ?
		}
	}
}
