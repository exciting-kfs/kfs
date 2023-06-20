use crate::{interrupt::InterruptFrame, pr_info, pr_warn};

#[no_mangle]
pub extern "C" fn handle_timer_impl(frame: InterruptFrame) {
	pr_warn!("timer");
	pr_info!("{}", frame);

	loop {}
}
