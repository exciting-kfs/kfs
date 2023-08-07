//! serial (COM1) driver
//! - see also <https://en.wikibooks.org/w/index.php?title=Serial_Programming/8250_UART_Programming>

use alloc::sync::Arc;
use kfs_macro::interrupt_handler;

use crate::{
	interrupt::{
		apic::{
			io::{set_irq_mask, SERIAL_COM1_IRQ},
			local::LOCAL_APIC,
		},
		InterruptFrame,
	},
	io::pmio::Port,
	pr_warn,
	process::task::{State, Task, CURRENT},
	scheduler::sleep::{sleep_and_yield, wake_up},
};

#[derive(Debug)]
pub enum Error {
	SelfDiagnosisFailed,
}

type Result = core::result::Result<(), Error>;

const COM1_PORT: u16 = 0x3f8;
const COM2_PORT: u16 = 0x2f8;

pub static mut SERIAL_COM1: Serial = Serial::new(COM1_PORT);
pub static mut SERIAL_EXT_COM1: SerialExt = SerialExt::new(COM1_PORT, SERIAL_COM1_IRQ);

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

pub struct SerialExt {
	serial: Serial,
	waiting_task: Option<Arc<Task>>,
	irq_num: usize,
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
		Serial {
			data,
			interrupt_enable,
			fifo_control,
			line_control,
			modem_control,
			line_status,
			modem_status,
			scratch,
		}
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

	pub fn wait_readable(&self) {
		while !self.read_available() {}
	}

	pub fn get_byte(&self) -> Option<u8> {
		self.read_available().then_some(self.data.read_byte())
	}

	fn disable_interrupts(&self) {
		self.interrupt_enable.write_byte(0x00);
	}

	fn enable_interrupts(&self, ier: u8) {
		self.interrupt_enable.write_byte(ier);
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

impl SerialExt {
	pub const fn new(base: u16, irq_num: usize) -> Self {
		Self {
			serial: Serial::new(base),
			waiting_task: None,
			irq_num,
		}
	}

	pub fn waiting_task(&mut self) -> Option<Arc<Task>> {
		core::mem::take(&mut self.waiting_task)
	}

	pub fn init(&self) -> Result {
		// enable `Transmitter Holding Resister Empty` interrupt.
		self.serial.enable_interrupts(1 << 1);
		// buad rate = 38400
		self.serial.set_baud_rate_divisor(3);
		self.serial.init_line_control();
		self.serial.disable_fifo();

		return self.serial.self_diagnosis();
	}
}

impl core::fmt::Write for Serial {
	fn write_str(&mut self, s: &str) -> core::fmt::Result {
		for ch in s.bytes() {
			while !self.write_available() {}
			self.data.write_byte(ch);
		}
		Ok(())
	}
}

impl core::fmt::Write for SerialExt {
	fn write_str(&mut self, s: &str) -> core::fmt::Result {
		let mut i = 0;
		while i < s.len() {
			let ch = s.as_bytes()[i];
			if self.serial.write_available() {
				self.serial.data.write_byte(ch);
				i += 1;
			} else {
				set_irq_mask(self.irq_num, false).expect("setting mask of IRQ");
				self.waiting_task = Some(unsafe { CURRENT.get_mut() }.clone());
				sleep_and_yield(State::Sleeping);
			}
		}
		Ok(())
	}
}

pub fn init() -> Result {
	unsafe { SERIAL_COM1.init() }
}

pub fn ext_init() -> Result {
	unsafe { SERIAL_EXT_COM1.init() }
}

#[interrupt_handler]
pub extern "C" fn handle_serial_impl(_frame: InterruptFrame) {
	pr_warn!("serial");

	if let Some(task) = unsafe { SERIAL_EXT_COM1.waiting_task() } {
		wake_up(&task, State::Sleeping)
	}

	set_irq_mask(SERIAL_COM1_IRQ, true).expect("setting mask of IRQ"); // irq num?
	LOCAL_APIC.end_of_interrupt();
}
