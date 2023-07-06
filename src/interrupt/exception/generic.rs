use crate::interrupt::InterruptFrame;
use crate::{pr_err, pr_info, register};

#[no_mangle]
pub extern "C" fn handle_divide_error_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): DIVIDE ERROR");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_page_fault_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): PAGE FAULT");
	pr_info!("{}", frame);
	let cr2 = register!("cr2");
	pr_info!("note: while accessing {:#0x}", cr2);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_invalid_opcode_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): INVALID OPCODE");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_general_protection_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): GENERAL PROTECTION");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_double_fault_impl(frame: InterruptFrame) {
	pr_err!("Exception(abort): DOUBLE FAULT");
	pr_info!("{}", frame);

	loop {}
}
