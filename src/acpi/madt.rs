use acpi::{madt::Madt, platform::interrupt::Apic, sdt::Signature, InterruptModel};

use crate::{acpi::ACPI_TABLES, sync::singleton::Singleton};

pub static IOAPIC_INFO: Singleton<Apic> = Singleton::uninit();

pub fn init() {
	let madt = unsafe {
		ACPI_TABLES
			.lock()
			.get_sdt::<Madt>(Signature::MADT)
			.expect("madt")
			.expect("madt")
			.virtual_start()
			.as_mut()
	};

	let (interrupt_model, _) = madt.parse_interrupt_model().expect("parsing madt");

	match interrupt_model {
		InterruptModel::Apic(apic) => unsafe {
			IOAPIC_INFO.write(apic);
		},
		_ => panic!("unsupported interrupt model."),
	}
}
