use crate::{
	interrupt::apic::{apic_local_pbase, apic_local_vbase},
	mm::{
		constant::PAGE_MASK,
		page::{arch::init::VMEMORY, get_vmemory_map, map_mmio, PageFlag, VMemory},
		util::addr_to_pfn,
	},
	util::arch::msr::Msr,
};

/// # Description
///
/// - mapping apic register page set by BIOS.
/// - Msr: MTRR_PHYSBASE(0 ~ 9: 200H.step(2))
/// - Msr: MTRR_PHYSMASK(0 ~ 9: 201H.step(2))
///
/// # Allocation
/// - page table.
pub(super) unsafe fn mapping_apic_registers() -> Result<(), ApicError> {
	// check apic register page is set uncacheable.
	let base_val = Msr::new(0x200).read();
	let mask_val = Msr::new(0x201).read();
	let base = base_val.low & PAGE_MASK;
	let mask = mask_val.low & PAGE_MASK;
	let apic_paddr = apic_local_pbase();

	if base & mask != apic_paddr & mask {
		return Err(ApicError::Cacheable);
	}

	// mapping apic register page.
	let apic_vaddr = apic_local_vbase();
	let flags = PageFlag::Global | PageFlag::Write | PageFlag::Present;
	map_mmio(apic_vaddr, apic_paddr, flags).map_err(|_| ApicError::Alloc)?;

	let vm = get_vmemory_map();
	VMEMORY.write(VMemory {
		local_apic_pfn: addr_to_pfn(apic_vaddr),
		..vm
	});

	Ok(())
}

pub(super) enum ApicError {
	Alloc,
	Cacheable,
}

impl core::fmt::Debug for ApicError {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		match self {
			Self::Alloc => write!(f, "out of memory."),
			Self::Cacheable => write!(f, "apic register page must be uncacheable."),
		}
	}
}
