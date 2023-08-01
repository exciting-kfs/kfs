use kfs_macro::interrupt_handler;

use crate::console::{console_manager_work, CONSOLE_MANAGER};
use crate::driver::ps2::keyboard::{get_raw_scancode, into_key_event};
use crate::input::{self, key_event::Code};
use crate::interrupt::{apic::local::LOCAL_APIC, InterruptFrame};
use crate::scheduler::work::{schedule_fast_work, wakeup_fast_woker};

#[interrupt_handler]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	let code = get_raw_scancode();

	into_key_event(code as u8).map(|ev| {
		if ev.key == Code::Backtick && ev.pressed() {
			panic!("BACKTICK PRESSED!!");
		}
		input::keyboard::change_state(ev);

		unsafe {
			if ev.pressed() {
				CONSOLE_MANAGER.assume_init_ref().update(ev.key);
			}
		}
		schedule_fast_work(console_manager_work, ev);
		wakeup_fast_woker();
	});

	LOCAL_APIC.end_of_interrupt();
}
