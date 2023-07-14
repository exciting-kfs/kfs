use kfs_macro::context;

use crate::{
	console::console_manager_work,
	driver::ps2::keyboard::{get_raw_scancode, into_key_event},
	input::{self, key_event::Code},
	interrupt::{apic::local::LOCAL_APIC, InterruptFrame},
	pr_warn,
	scheduler::work::{schedule_fast_work, wakeup_fast_woker},
};

#[context(hw_irq)]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	pr_warn!("keyboard");
	let code = get_raw_scancode();

	into_key_event(code as u8).map(|ev| {
		if ev.key == Code::Backtick && ev.pressed() {
			panic!("BACKTICK PRESSED!!");
		}
		input::keyboard::change_state(ev);

		schedule_fast_work(console_manager_work, ev);
		wakeup_fast_woker();
	});

	LOCAL_APIC.end_of_interrupt();
}
