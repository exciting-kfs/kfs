use kfs_macro::interrupt_handler;

use crate::config::CONSOLE_COUNTS;
use crate::console::{console_screen_draw, CONSOLE_MANAGER};
use crate::driver::ps2::keyboard::{get_raw_scancode, into_key_event};
use crate::input::key_event::KeyKind;
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
				let cm = CONSOLE_MANAGER.assume_init_ref();
				cm.update(ev.key);

				if let KeyKind::Function(v) = ev.identify() {
					let idx = v.index() as usize;

					if idx < CONSOLE_COUNTS {
						cm.set_foreground(idx);
					}
				}
			}
		}
		schedule_fast_work(console_screen_draw, ());
		wakeup_fast_woker();
	});

	LOCAL_APIC.end_of_interrupt();
}
