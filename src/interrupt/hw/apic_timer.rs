use crate::interrupt::{apic::local::LOCAL_APIC, InterruptFrame};
use crate::process::context::{cpu_context, InContext};
use crate::process::task::yield_now;
use crate::sync::cpu_local::CpuLocal;

#[no_mangle]
pub unsafe extern "C" fn handle_timer_impl(_frame: InterruptFrame) {
	*JIFFIES.get_mut() += 1;
	LOCAL_APIC.end_of_interrupt();

	if let InContext::PreemptDisabled = cpu_context() {
		return;
	}

	yield_now();
}

static JIFFIES: CpuLocal<usize> = CpuLocal::new(0);

pub fn jiffies() -> usize {
	unsafe { *JIFFIES.get_mut() }
}
