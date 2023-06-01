mod io;
mod local;

use crate::interrupt::apic::io::REDIR_TABLE_COUNT;
use crate::{acpi::APIC_INFO, pr_info, sync::singleton::Singleton, util::arch::msr::Msr};

pub use io::pbase as io_pbase;
pub use io::vbase as io_vbase;
pub use local::pbase as local_pbase;
pub use local::vbase as local_vbase;

static MSR_APIC_BASE: Singleton<Msr> = Singleton::new(Msr::new(0x1b));

pub fn init() {
	io::init();
}

pub fn print_local() {
	pr_info!("\n**** apic local register ****");
	pr_info!("vbase: {:x}", local::vbase());

	for r in local::Register::iter() {
		pr_info!("{}: {:x?}", r, r.read())
	}
}

pub fn print_io() {
	pr_info!("\n**** apic io register ****");

	for id in 0..APIC_INFO.lock().io_apics.iter().count() {
		pr_info!("vbase: {:x}", io::vbase(id));
		pr_info!("ID: {:?}", io::read(id, io::RegKind::ID));
		pr_info!("Version: {:?}", io::read(id, io::RegKind::VERSION));
		pr_info!("ArbID: {:?}", io::read(id, io::RegKind::ArbitrationID));
		pr_info!("Redirection Table:");

		for i in 0..REDIR_TABLE_COUNT {
			pr_info!(
				"\t {:02}: {:x?}",
				i,
				io::read(id, io::RegKind::RedirectionTable(i as u8))
			);
		}
	}
}
