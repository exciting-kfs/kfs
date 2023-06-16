use acpi::AcpiTables;

use self::handler::AcpiH;

mod fadt;
mod handler;
mod madt;

pub static mut RSDT_PADDR: usize = 0;
pub use fadt::IAPC_BOOT_ARCH;
pub use madt::{IOAPIC_INFO, PROCESSOR_INFO};

pub fn init() {
	unsafe {
		let acpi_table = AcpiTables::from_rsdt(AcpiH, 0, RSDT_PADDR).expect("acpi table");
		madt::init(&acpi_table);
		fadt::init(&acpi_table);
	};
}
