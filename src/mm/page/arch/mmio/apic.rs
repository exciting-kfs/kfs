use alloc::vec::Vec;

use crate::{
	acpi::IOAPIC_INFO,
	interrupt::apic::{self, LAPIC_PBASE, MSR_APIC_BASE},
	mm::{
		constant::PAGE_MASK,
		page::{arch::init::VMEMORY, get_vmemory_map, map_mmio, PageFlag, VMemory},
		util::{addr_to_pfn, phys_to_virt},
	},
	util::arch::msr::Msr,
};

/// # Description
/// - mapping local apic register page set by BIOS.
///
/// # Allocation
/// - page table.
fn mapping_local_apic_registers() -> Result<(), ApicError> {
	let apic_paddr = apic::local_pbase();
	is_uncacheable_page(apic_paddr).map_err(|_| ApicError::Cacheable("local"))?;

	// mapping local apic register page.
	let apic_vaddr = apic::local_vbase();
	let flags = PageFlag::Global | PageFlag::Write | PageFlag::Present;
	map_mmio(apic_vaddr, apic_paddr, flags).map_err(|_| ApicError::Alloc)?;

	// recording local apic pfn at VMemory.
	unsafe {
		let vm = get_vmemory_map();
		VMEMORY.write(VMemory {
			local_apic_pfn: addr_to_pfn(apic_vaddr),
			..vm
		});
	}

	Ok(())
}

/// # Description
/// - mapping io apic register page set by BIOS.
/// - IOREGSEL(index) 0xfec0_xy00
/// - IOWIN(data)     0xfec0_xy10
/// - `xy` determined by the x and y field in the APIC Base Address Relocation Register located in PIIX3(south bridge).
///
/// #Allocation
/// - page table.
fn mapping_io_apic_registers() -> Result<(), ApicError> {
	let mut pfns = Vec::new();

	// mapping io apic register page.
	for io_apic in IOAPIC_INFO.io_apics.iter() {
		is_uncacheable_page(io_apic.address as usize).map_err(|_| ApicError::Cacheable("io"))?;
		let paddr = io_apic.address as usize;
		let vaddr = phys_to_virt(paddr);
		let flags = PageFlag::Global | PageFlag::Write | PageFlag::Present;
		map_mmio(vaddr, paddr, flags).map_err(|_| ApicError::Alloc)?; // FIXME hmm.. cleanup?
		pfns.push(addr_to_pfn(vaddr));
	}

	// recording io apic pfn at VMemory.
	unsafe {
		let vm = get_vmemory_map();
		VMEMORY.write(VMemory {
			io_apic_pfn: pfns,
			..vm
		});
	}

	Ok(())
}

/// # Description
/// - Msr: MTRR_PHYSBASE(0 ~ 9: 200H.step(2))
/// - Msr: MTRR_PHYSMASK(0 ~ 9: 201H.step(2))
fn is_uncacheable_page(paddr: usize) -> Result<(), ()> {
	let base_val = Msr::new(0x200).read(); // FIXME hmm.. whole MTRR?
	let mask_val = Msr::new(0x201).read();
	let base = base_val.low & PAGE_MASK;
	let mask = mask_val.low & PAGE_MASK;

	if base & mask != paddr & mask {
		Err(())
	} else {
		Ok(())
	}
}

pub(super) enum ApicError {
	Alloc,
	Cacheable(&'static str),
}

impl core::fmt::Debug for ApicError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Alloc => write!(f, "out of memory."),
			Self::Cacheable(s) => write!(f, "{} apic register page must be uncacheable.", s),
		}
	}
}

pub fn init() {
	unsafe {
		LAPIC_PBASE = MSR_APIC_BASE.read().low & PAGE_MASK;
	}
	mapping_local_apic_registers().expect("mapping local apic");
	mapping_io_apic_registers().expect("mapping io apic");
}
