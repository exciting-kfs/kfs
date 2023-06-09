use core::ptr::{self, addr_of};

use acpi::{
	fadt::{Fadt, IaPcBootArchFlags},
	sdt::Signature,
	AcpiTables,
};

use crate::util::lazy_constant::LazyConstant;

use super::handler::AcpiH;

pub static IAPC_BOOT_ARCH: LazyConstant<IaPcBootArchFlags> = LazyConstant::uninit();

pub unsafe fn init(acpi_tables: &AcpiTables<AcpiH>) {
	let mapping = acpi_tables
		.get_sdt::<Fadt>(Signature::FADT)
		.expect("fadt")
		.expect("fadt");

	let fadt = mapping.virtual_start().as_mut();

	let value: IaPcBootArchFlags = ptr::read_unaligned(addr_of!(fadt.iapc_boot_arch));
	IAPC_BOOT_ARCH.write(value);
}
