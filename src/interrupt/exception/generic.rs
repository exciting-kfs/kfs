use crate::interrupt::InterruptFrame;
use crate::process::exit::exit_with_signal;
use crate::signal::sig_num::SigNum;
use crate::{pr_err, pr_info};

#[no_mangle]
pub extern "C" fn handle_divide_error_impl(frame: InterruptFrame) {
	if frame.is_user() {
		exit_with_signal(SigNum::FPE);
	}

	pr_err!("Exception(fault): DIVIDE ERROR");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_invalid_opcode_impl(frame: InterruptFrame) {
	if frame.is_user() {
		exit_with_signal(SigNum::ILL);
	}

	pr_err!("Exception(fault): INVALID OPCODE");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_general_protection_impl(frame: InterruptFrame) {
	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	pr_err!("Exception(fault): GENERAL PROTECTION");
	pr_info!("{}", frame);

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_double_fault_impl(frame: InterruptFrame) {
	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	pr_err!("Exception(abort): DOUBLE FAULT");
	pr_info!("{}", frame);

	loop {}
}
