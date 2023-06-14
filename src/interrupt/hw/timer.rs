use crate::interrupt::{apic::end_of_interrupt, InterruptFrame};
use crate::{pr_info, pr_warn};

#[no_mangle]
pub extern "C" fn handle_timer_impl(frame: InterruptFrame) {
	pr_warn!("timer");
	pr_info!("{}", frame);

	end_of_interrupt();
	loop {}
}
