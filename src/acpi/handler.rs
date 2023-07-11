use core::{mem::size_of, ptr::NonNull};

use acpi::AcpiHandler;

use crate::mm::alloc::virt::{io_allocate, io_deallocate};
use crate::mm::{constant::*, util::*};

#[derive(Clone)]
pub struct AcpiH;

impl AcpiHandler for AcpiH {
	unsafe fn map_physical_region<T>(
		&self,
		physical_address: usize,
		size: usize,
	) -> acpi::PhysicalMapping<Self, T> {
		let phys_map_start = physical_address & PAGE_MASK;
		let phys_map_end = next_align(physical_address + size, PAGE_SIZE);

		let mapped_length = phys_map_end - phys_map_start;

		let virt_map_start = io_allocate(phys_map_start, mapped_length / PAGE_SIZE)
			.expect("acpi: OOM")
			.as_ptr()
			.cast::<u8>() as usize;

		let virtual_address =
			NonNull::new_unchecked((virt_map_start + (physical_address & !PAGE_MASK)) as *mut T);

		acpi::PhysicalMapping::new(
			physical_address,
			virtual_address,
			size_of::<T>(),
			mapped_length,
			self.clone(),
		)
	}

	fn unmap_physical_region<T>(region: &acpi::PhysicalMapping<Self, T>) {
		io_deallocate(
			region.virtual_start().as_ptr() as usize & PAGE_MASK,
			region.mapped_length() / PAGE_SIZE,
		)
	}
}
