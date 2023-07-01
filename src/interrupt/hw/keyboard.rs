use crate::{
	interrupt::{apic::end_of_interrupt, InterruptFrame},
	io::pmio::Port,
	pr_info, pr_warn,
	process::context::{context_switch, InContext},
};
use kfs_macro::context;

#[context(irq_disabled)]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	pr_warn!("keyboard");
	let c = Port::new(0x60).read_byte();

	pr_info!("read from keyboard: {}", c);

	end_of_interrupt();
}
