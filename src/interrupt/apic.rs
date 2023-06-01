mod local;

use crate::{
	mm::{constant::PAGE_MASK, util::phys_to_virt},
	pr_info,
	sync::singleton::Singleton,
	util::arch::msr::Msr,
};

static MSR_APIC_BASE: Singleton<Msr> = Singleton::new(Msr::new(0x1b));

pub fn apic_local_pbase() -> usize {
	MSR_APIC_BASE.lock().read().low & PAGE_MASK
}

pub fn apic_local_vbase() -> usize {
	phys_to_virt(apic_local_pbase())
}

pub fn init() {}

pub fn print_local() {
	let base = apic_local_vbase();
	pr_info!("apic local base: {:x}", base);
	pr_info!("apic local register:");

	for r in local::Register::iter() {
		pr_info!("{}: {:x?}", r, r.read())
	}
}
