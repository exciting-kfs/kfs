//! serial (COM1) driver
//! - see alsohttps://en.wikibooks.org/w/index.php?title=Serial_Programming/8250_UART_Programming

mod com1 {
	use crate::io::pmio::Port;

	const COM1_PORT: u16 = 0x3f8;

	pub static DATA: Port = Port::new(COM1_PORT);
	pub static INTERRUPT_ENABLE: Port = Port::new(COM1_PORT + 1);
	pub static FIFO_CONTROL: Port = Port::new(COM1_PORT + 2);
	pub static LINE_CONTROL: Port = Port::new(COM1_PORT + 3);
	pub static MODEM_CONTROL: Port = Port::new(COM1_PORT + 4);
	pub static LINE_STATUS: Port = Port::new(COM1_PORT + 5);
	pub static MODEM_STATUS: Port = Port::new(COM1_PORT + 6);
	pub static SCRATCH: Port = Port::new(COM1_PORT + 7);
}

#[derive(Debug)]
pub enum Error {
	SelfDiagnosisFailed,
}

type Result = core::result::Result<(), Error>;

pub struct Serial;
impl core::fmt::Write for Serial {
	fn write_str(&mut self, s: &str) -> core::fmt::Result {
		for ch in s.bytes() {
			while !write_available() {}
			com1::DATA.write_byte(ch);
		}

		Ok(())
	}
}

pub fn write_available() -> bool {
	com1::LINE_STATUS.read_byte() & (1 << 5) != 0
}

pub fn read_available() -> bool {
	com1::LINE_STATUS.read_byte() & (1 << 0) != 0
}

pub fn get_byte() -> Option<u8> {
	read_available().then_some(com1::DATA.read_byte())
}

fn disable_interrupts() {
	com1::INTERRUPT_ENABLE.write_byte(0x00);
}

fn set_divisor_latch(turn_on: bool) {
	let old = com1::LINE_CONTROL.read_byte();

	let mask = 1 << 7;
	let new = match turn_on {
		true => old | mask,
		false => old & !mask,
	};

	com1::LINE_CONTROL.write_byte(new);
}

/// set baud rate divisor.
///  - if divisor is 1 then baud rate will be 115200.
///  - if divisor is 2 then baud rate will be 57600.
///  - and so on...
fn set_baud_rate_divisor(divisor: u16) {
	let hi = (divisor >> 8) as u8;
	let lo = (divisor & ((1 << 8) - 1)) as u8;

	// when divisor latch is on
	// - DATA port is divisor's LOW 8bit
	// - INTERRUPT_EABLE port is divisor's HIGH 8bit
	set_divisor_latch(true);
	com1::DATA.write_byte(lo);
	com1::INTERRUPT_ENABLE.write_byte(hi);
	set_divisor_latch(false);
}

fn init_line_control() {
	// 8bit / no parity / one stop bit.
	com1::LINE_CONTROL.write_byte(0x03);
}

fn disable_fifo() {
	// disable and clear fifo buffer.
	com1::LINE_CONTROL.write_byte(0x06);
}

fn self_diagnosis() -> Result {
	// set RTS, loopback, AO1, AO2
	com1::MODEM_CONTROL.write_byte(0x1E);
	com1::DATA.write_byte(0x42);
	if com1::DATA.read_byte() != 0x42 {
		return Err(Error::SelfDiagnosisFailed);
	}
	// set RTS, DTR, AO1, AO2
	com1::MODEM_CONTROL.write_byte(0x0f);
	Ok(())
}

pub fn init_serial() -> Result {
	disable_interrupts();
	// buad rate = 38400
	set_baud_rate_divisor(3);
	init_line_control();
	disable_fifo();

	return self_diagnosis();
}
