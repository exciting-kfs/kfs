pub mod control;
pub mod keyboard;

use kfs_macro::interrupt_handler;

use crate::driver::apic::local::LOCAL_APIC;
use crate::driver::ps2::keyboard::{get_raw_scancode, into_key_event};
use crate::driver::terminal::{console_screen_draw, get_foreground_tty, set_foreground_tty};
use crate::input::{
	self,
	key_event::{Code, KeyKind},
};
use crate::interrupt::InterruptFrame;
use crate::io::ChWrite;
use crate::pr_err;
use crate::scheduler::work::schedule_fast_work;
use crate::{acpi::IAPC_BOOT_ARCH, io::pmio::Port};

fn wait_then_write_byte(port: &Port, byte: u8) {
	while control::test_status_now(control::Status::IBF) {}
	port.write_byte(byte)
}

/// initilize PS/2 controller and device.
pub fn init() -> Result<(), ()> {
	check_ps2_existence()?;
	disable_devices();

	// drain all key event.
	while keyboard::available() {
		keyboard::poll_raw_scancode();
	}

	set_config();
	self_test()?;
	enable_keyboard()?;

	Ok(())
}

fn disable_devices() {
	// disable second ps/2 device (mouse)
	control::CONTROL_PORT.write_byte(0xa7);

	// disable first ps/2 device (keyboard)
	control::CONTROL_PORT.write_byte(0xad);
}

fn set_config() {
	// read current config
	wait_then_write_byte(&control::CONTROL_PORT, 0x20);
	let config = keyboard::wait_raw_scancode();

	// enable IRQ, translation on.
	let new_config = (config & !0b11) | (1 << 6) | 1;
	wait_then_write_byte(&control::CONTROL_PORT, 0x60);
	wait_then_write_byte(&keyboard::KEYBOARD_PORT, new_config);
}

fn self_test() -> Result<(), ()> {
	// self-test (PS/2 controller)
	wait_then_write_byte(&control::CONTROL_PORT, 0xaa);
	let result = keyboard::wait_raw_scancode();
	if result != 0x55 {
		return Err(());
	}

	// self-test (PS/2 first device)
	wait_then_write_byte(&control::CONTROL_PORT, 0xab);
	let result = keyboard::wait_raw_scancode();
	if result != 0x00 {
		return Err(());
	}

	Ok(())
}

fn enable_keyboard() -> Result<(), ()> {
	// enable PS/2 first device
	wait_then_write_byte(&control::CONTROL_PORT, 0xae);

	// reset keyboard.
	wait_then_write_byte(&keyboard::KEYBOARD_PORT, 0xff);
	let result = keyboard::wait_raw_scancode();
	if result != 0xfa {
		return Err(());
	}
	Ok(())
}

fn check_ps2_existence() -> Result<(), ()> {
	if !IAPC_BOOT_ARCH.motherboard_implements_8042() {
		Ok(())
	} else {
		Err(())
	}
}

#[interrupt_handler]
pub extern "C" fn handle_keyboard_impl(_frame: InterruptFrame) {
	let code = get_raw_scancode();

	into_key_event(code as u8).map(|ev| {
		if ev.key == Code::Backtick && ev.pressed() {
			pr_err!("BACKTICK PRESSED!!");
		}
		input::keyboard::change_state(ev);

		if ev.pressed() {
			let tty = get_foreground_tty();

			if let KeyKind::Function(v) = ev.identify() {
				let idx = v.index() as usize;

				set_foreground_tty(idx);
			} else {
				let _ = tty.lock_tty().write_one(ev.key);
			}
		}

		schedule_fast_work(console_screen_draw, ());
		// wakeup_fast_woker();
	});

	LOCAL_APIC.end_of_interrupt();
}
