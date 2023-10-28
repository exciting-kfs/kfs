use crate::interrupt::InterruptFrame;
use crate::process::exit::exit_with_signal;
use crate::process::signal::sig_num::SigNum;
use crate::{pr_err, pr_info};

#[no_mangle]
pub extern "C" fn handle_divide_error_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): DIVIDE ERROR");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::FPE);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_invalid_opcode_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): INVALID OPCODE");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::ILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_general_protection_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): GENERAL PROTECTION");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_double_fault_impl(frame: InterruptFrame) {
	pr_err!("Exception(abort): DOUBLE FAULT");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_control_protection_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): CONTROL PROTECTION");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_stack_fault_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): STACK FAULT");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_not_present_impl(frame: InterruptFrame) {
	pr_err!("Exception(fault): SEGMENT NOT PRESENT");
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}

#[no_mangle]
pub extern "C" fn handle_tss_fault_impl(frame: InterruptFrame) {
	pr_info!("{}", frame);

	if frame.is_user() {
		exit_with_signal(SigNum::KILL);
	}

	loop {}
}
