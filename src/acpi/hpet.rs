use acpi::{AcpiTables, HpetInfo};

use super::handler::AcpiH;

pub static mut HPET_BASE: usize = 0;

pub fn init(acpi_tables: &AcpiTables<AcpiH>) {
	let hpet_info = HpetInfo::new(acpi_tables).expect("HPET");

	unsafe { HPET_BASE = hpet_info.base_address };
}
