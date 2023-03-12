pub mod control;
pub mod keyboard;

use crate::io::pmio::Port;

fn wait_then_write_byte(port: &Port, byte: u8) {
	while control::test_status_now(control::Status::IBF) {}
	port.write_byte(byte)
}

/// initilize PS/2 controller and device.
pub fn init_ps2() -> Result<(), ()> {
	// disable second ps/2 device (mouse)
	control::CONTROL_PORT.write_byte(0xa7);

	// disable first ps/2 device (keyboard)
	control::CONTROL_PORT.write_byte(0xad);

	// drain all key event.
	while keyboard::available() {
		keyboard::get_raw_scancode();
	}

	// read current config
	wait_then_write_byte(&control::CONTROL_PORT, 0x20);
	let config = keyboard::wait_raw_scancode();

	// disable IRQ, translation on.
	let new_config = (config & !0b11) | (1 << 6);
	wait_then_write_byte(&control::CONTROL_PORT, 0x60);
	wait_then_write_byte(&keyboard::KEYBOARD_PORT, new_config);

	// self-test (PS/2 controller)
	wait_then_write_byte(&control::CONTROL_PORT, 0xaa);
	let mut result = keyboard::wait_raw_scancode();
	if result != 0x55 {
		return Err(());
	}

	// self-test (PS/2 first device)
	wait_then_write_byte(&control::CONTROL_PORT, 0xab);
	result = keyboard::wait_raw_scancode();
	if result != 0x00 {
		return Err(());
	}

	// enable PS/2 first device
	wait_then_write_byte(&control::CONTROL_PORT, 0xae);

	Ok(())
}
