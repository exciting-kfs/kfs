use acpi::AcpiTables;

use crate::sync::singleton::Singleton;

use self::handler::AcpiH;

mod handler;
mod madt;

pub static mut RSDT_PADDR: usize = 0;
pub static ACPI_TABLES: Singleton<AcpiTables<AcpiH>> = Singleton::uninit();
pub use madt::IOAPIC_INFO;

pub fn init() {
	unsafe {
		let acpi_table = AcpiTables::from_rsdt(AcpiH, 0, RSDT_PADDR).expect("acpi table");
		ACPI_TABLES.write(acpi_table)
	};

	madt::init();
}
