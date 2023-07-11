pub mod control;
pub mod keyboard;

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
