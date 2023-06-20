use crate::{interrupt::InterruptFrame, pr_info, pr_warn};

#[no_mangle]
pub extern "C" fn handle_keyboard_impl(frame: InterruptFrame) {
	pr_warn!("keyboard");
	pr_info!("{}", frame);

	loop {}
}
