use acpi::{madt::Madt, platform::interrupt::Apic, sdt::Signature, AcpiTables, InterruptModel};

use crate::util::lazy_constant::LazyConstant;

use super::handler::AcpiH;

pub static IOAPIC_INFO: LazyConstant<Apic> = LazyConstant::uninit();

pub unsafe fn init(acpi_tables: &AcpiTables<AcpiH>) {
	let mapping = acpi_tables
		.get_sdt::<Madt>(Signature::MADT)
		.expect("madt")
		.expect("madt");

	let madt = mapping.virtual_start().as_mut();
	let (interrupt_model, _) = madt.parse_interrupt_model().expect("parsing madt");

	match interrupt_model {
		InterruptModel::Apic(apic) => unsafe {
			IOAPIC_INFO.write(apic);
		},
		_ => panic!("unsupported interrupt model."),
	}
}
