use crate::interrupt::{apic::local::LOCAL_APIC, InterruptFrame};
use crate::io::pmio::Port;
use crate::{pr_info, pr_warn};

#[no_mangle]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	pr_warn!("keyboard");
	let c = Port::new(0x60).read_byte();

	pr_info!("read from keyboard: {}", c);

	LOCAL_APIC.end_of_interrupt();
}
