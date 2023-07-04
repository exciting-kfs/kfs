use alloc::boxed::Box;
use kfs_macro::context;

use crate::{
	console::console_manager_tasklet,
	driver::ps2::keyboard::{get_raw_scancode, into_key_event},
	input::{self, key_event::Code},
	interrupt::{
		apic::local::LOCAL_APIC,
		tasklet::{do_tasklet_timeout, schedule_tasklet, Tasklet},
		InterruptFrame,
	},
	pr_warn,
};

#[context(hw_irq)]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	pr_warn!("keyboard");
	let code = get_raw_scancode();
	let event = into_key_event(code as u8);

	if event.key == Code::Backtick && event.pressed() {
		panic!("BACKTICK PRESSED!!");
	}
	input::keyboard::change_state(event);

	let tasklet = Tasklet::new(console_manager_tasklet, Box::new(event));
	schedule_tasklet(tasklet);
	do_tasklet_timeout();

	LOCAL_APIC.end_of_interrupt();
}
