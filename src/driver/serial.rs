//! serial (COM1) driver
//! - see also https://en.wikibooks.org/w/index.php?title=Serial_Programming/8250_UART_Programming
use crate::io::pmio::Port;

#[derive(Debug)]
pub enum Error {
	SelfDiagnosisFailed,
}

type Result = core::result::Result<(), Error>;

const COM1_PORT: u16 = 0x3f8;
const COM2_PORT: u16 = 0x2f8;

pub static mut COM1: Serial = Serial::new(COM1_PORT);
pub static mut COM2: Serial = Serial::new(COM2_PORT);

pub struct Serial {
	data: Port,
	interrupt_enable: Port,
	fifo_control: Port,
	line_control: Port,
	modem_control: Port,
	line_status: Port,
	modem_status: Port,
	scratch: Port,
}

impl Serial {
	pub const fn new(base: u16) -> Self {
		let data = Port::new(base);
		let interrupt_enable = Port::new(base + 1);
		let fifo_control = Port::new(base + 2);
		let line_control = Port::new(base + 3);
		let modem_control = Port::new(base + 4);
		let line_status = Port::new(base + 5);
		let modem_status = Port::new(base + 6);
		let scratch = Port::new(base + 7);
		Serial { data, interrupt_enable, fifo_control, line_control, modem_control, line_status, modem_status, scratch }
	}

	pub fn init(&self) -> Result {
		self.disable_interrupts();
		// buad rate = 38400
		self.set_baud_rate_divisor(3);
		self.init_line_control();
		self.disable_fifo();

		return self.self_diagnosis();
	}

	pub fn write_available(&self) -> bool {
		self.line_status.read_byte() & (1 << 5) != 0
	}
	
	pub fn read_available(&self) -> bool {
		self.line_status.read_byte() & (1 << 0) != 0
	}


	pub fn get_byte(&self) -> Option<u8> {
		self.read_available().then_some(self.data.read_byte())
	}

	fn disable_interrupts(&self) {
		self.interrupt_enable.write_byte(0x00);
	}

	fn set_divisor_latch(&self, turn_on: bool) {
		let old = self.line_control.read_byte();

		let mask = 1 << 7;
		let new = match turn_on {
			true => old | mask,
			false => old & !mask,
		};

		self.line_control.write_byte(new);
	}

	/// set baud rate divisor.
	///  - if divisor is 1 then baud rate will be 115200.
	///  - if divisor is 2 then baud rate will be 57600.
	///  - and so on...
	fn set_baud_rate_divisor(&self, divisor: u16) {
		let hi = (divisor >> 8) as u8;
		let lo = (divisor & ((1 << 8) - 1)) as u8;

		// when divisor latch is on
		// - DATA port is divisor's LOW 8bit
		// - INTERRUPT_EABLE port is divisor's HIGH 8bit
		self.set_divisor_latch(true);
		self.data.write_byte(lo);
		self.interrupt_enable.write_byte(hi);
		self.set_divisor_latch(false);
	}

	fn init_line_control(&self) {
		// 8bit / no parity / one stop bit.
		self.line_control.write_byte(0x03);
	}

	fn disable_fifo(&self) {
		// disable and clear fifo buffer.
		self.line_control.write_byte(0x06);
	}

	fn self_diagnosis(&self) -> Result {
		// set RTS, loopback, AO1, AO2
		self.modem_control.write_byte(0x1E);
		self.data.write_byte(0x42);
		if self.data.read_byte() != 0x42 {
			return Err(Error::SelfDiagnosisFailed);
		}
		// set RTS, DTR, AO1, AO2
		self.modem_control.write_byte(0x0f);
		Ok(())
	}

}

unsafe impl Sync for Serial {}

impl core::fmt::Write for Serial {
	fn write_str(&mut self, s: &str) -> core::fmt::Result {
		for ch in s.bytes() {
			while !self.write_available() {}
			self.data.write_byte(ch);
		}

		Ok(())
	}
}


pub fn init_serial() {
	unsafe {
		COM1.init().expect("failed to init COM1 serial port");
		COM2.init().expect("failed to init COM2 serial port");
	}
}
