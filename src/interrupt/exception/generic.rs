use kfs_macro::context;

use crate::interrupt::InterruptFrame;
use crate::process::context::{context_switch, InContext};
use crate::{pr_err, pr_info, register};

#[context(irq_disabled)]
pub extern "C" fn handle_divide_error_impl(frame: &InterruptFrame) {
	pr_err!("Exception(fault): DIVIDE ERROR");
	pr_info!("{}", frame);

	loop {}
}

#[context(irq_disabled)]
pub extern "C" fn handle_page_fault_impl(frame: &InterruptFrame) {
	pr_err!("Exception(fault): PAGE FAULT");
	pr_info!("{}", frame);
	let cr2 = register!("cr2");
	pr_info!("note: while accessing {:#0x}", cr2);

	loop {}
}

#[context(irq_disabled)]
pub extern "C" fn handle_invalid_opcode_impl(frame: &InterruptFrame) {
	pr_err!("Exception(fault): INVALID OPCODE");
	pr_info!("{}", frame);

	loop {}
}

#[context(irq_disabled)]
pub extern "C" fn handle_general_protection_impl(frame: &InterruptFrame) {
	pr_err!("Exception(fault): GENERAL PROTECTION");
	pr_info!("{}", frame);

	loop {}
}

#[context(irq_disabled)]
pub extern "C" fn handle_double_fault_impl(frame: &InterruptFrame) {
	pr_err!("Exception(abort): DOUBLE FAULT");
	pr_info!("{}", frame);

	loop {}
}
