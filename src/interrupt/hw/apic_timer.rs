use kfs_macro::interrupt_handler;

use crate::interrupt::{apic::local::LOCAL_APIC, InterruptFrame};
use crate::process::context::yield_now;
use crate::process::task::CURRENT;
use crate::sync::cpu_local::CpuLocal;

#[interrupt_handler]
pub unsafe extern "C" fn handle_timer_impl(frame: InterruptFrame) {
	*JIFFIES.get_mut() += 1;
	LOCAL_APIC.end_of_interrupt();

	yield_now();

	if frame.is_user() {
		CURRENT
			.get_mut()
			.get_user_ext()
			.expect("user task")
			.signal
			.do_signal(&frame, 0);
	}
}

static JIFFIES: CpuLocal<usize> = CpuLocal::new(0);

pub fn jiffies() -> usize {
	unsafe { *JIFFIES.get_mut() }
}
