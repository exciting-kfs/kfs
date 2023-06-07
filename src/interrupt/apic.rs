mod local;

use crate::{pr_info, sync::singleton::Singleton, util::arch::msr::Msr};

pub use local::pbase as local_pbase;
pub use local::vbase as local_vbase;

static MSR_APIC_BASE: Singleton<Msr> = Singleton::new(Msr::new(0x1b));

pub fn init() {}

pub fn print_local() {
	pr_info!("\n**** apic local register ****");
	pr_info!("vbase: {:x}", local::vbase());

	for r in local::Register::iter() {
		pr_info!("{}: {:x?}", r, r.read())
	}
}
