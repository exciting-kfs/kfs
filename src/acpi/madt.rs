use acpi::{
	madt::{EntryHeader, Madt, MadtEntry, MultiprocessorWakeupEntry},
	platform::interrupt::Apic,
	sdt::Signature,
	AcpiTables, InterruptModel,
};

use crate::{pr_info, util::lazy_constant::LazyConstant};

use super::handler::AcpiH;

pub static IOAPIC_INFO: LazyConstant<Apic> = LazyConstant::uninit();

pub unsafe fn init(acpi_tables: &AcpiTables<AcpiH>) {
	let mapping = acpi_tables
		.get_sdt::<Madt>(Signature::MADT)
		.expect("madt")
		.expect("madt");

	let madt = mapping.virtual_start().as_mut();

	// madt.entries().for_each(|e| {
	// 	if let MadtEntry::MultiprocessorWakeup(et) = e {
	// 		let ptr = (et as *const MultiprocessorWakeupEntry as *mut EntryHeader).offset(1);
	// 		let ptr =
	// 	}
	// });

	let (interrupt_model, processor_info) = madt.parse_interrupt_model().expect("parsing madt");

	match processor_info {
		Some(info) => {
			pr_info!("bsp: {:?}", info.boot_processor);
			info.application_processors
				.iter()
				.enumerate()
				.for_each(|(i, ap)| {
					pr_info!("ap[{}]: {:?}", i, ap);
				})
		}
		None => {}
	}

	match interrupt_model {
		InterruptModel::Apic(apic) => unsafe {
			IOAPIC_INFO.write(apic);
		},
		_ => panic!("unsupported interrupt model."),
	}
}
