use crate::interrupt::InterruptFrame;
use crate::process::exit::exit_with_signal;
use crate::signal::sig_num::SigNum;
use crate::{pr_err, pr_info, register};

#[no_mangle]
pub extern "C" fn handle_page_fault_impl(frame: InterruptFrame) {
	if frame.is_user() {
		exit_with_signal(SigNum::SEGV);
	}

	// BUG

	let addr = register!("cr2");
	pr_err!("Exception(fault): PAGE FAULT");
	pr_info!("{}", frame);
	pr_info!("note: while accessing {:#0x}", addr);

	loop {}
}
