use acpi::AcpiTables;

use crate::pr_info;

use self::handler::AcpiH;

mod fadt;
mod handler;
mod hpet;
mod madt;

pub static mut RSDT_PADDR: usize = 0;
pub use fadt::IAPC_BOOT_ARCH;
pub use hpet::HPET_BASE;
pub use madt::IOAPIC_INFO;

pub fn init() {
	unsafe {
		let acpi_table = AcpiTables::from_rsdt(AcpiH, 0, RSDT_PADDR).expect("acpi table");
		let version = match acpi_table.revision == 0 {
			true => "1.0",
			false => "2.0+",
		};

		pr_info!("ACPI version: {}", version);
		madt::init(&acpi_table);
		fadt::init(&acpi_table);
		hpet::init(&acpi_table);
	};
}
