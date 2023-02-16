use crate::console::CONSOLE_MANAGER;
use core::fmt;

const BUFFER_SIZE: usize = 5;

pub struct DebugMessage {
	len: usize,
	pub buf: [u8; BUFFER_SIZE],
}

impl DebugMessage {
	const fn new() -> Self {
		DebugMessage {
			len: 0,
			buf: [0; BUFFER_SIZE],
		}
	}

	pub unsafe fn flush(&mut self) {
		CONSOLE_MANAGER.get().dmesg().write_buf(&self.buf, self.len);
		self.len = 0;
	}

	fn write_byte(&mut self, b: u8) {
		self.buf[self.len] = b;
		self.len += 1;
		if self.len >= BUFFER_SIZE {
			unsafe { self.flush() }
		}
	}
}

impl fmt::Write for DebugMessage {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		for b in s.bytes() {
			self.write_byte(b)
		}
		Ok(())
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		self.write_byte(c as u8);
		Ok(())
	}
}

pub static mut DMESG: DebugMessage = DebugMessage::new();

#[macro_export]
macro_rules! printkln {
	($($arg:tt)*) => {
		use core::fmt::Write;
		unsafe {
			let (res1, res2) = (
				$crate::printk::DMESG.write_fmt(core::format_args!($($arg)*)),
				$crate::printk::DMESG.write_char(b'\n' as char),
			);
			if let (Err(_), Err(_)) =  (res1, res2){
				panic!("printk failed");
			}
		}
	};
}

#[macro_export]
macro_rules! printk {
	($($arg:tt)*) => {
		use core::fmt::Write;
		unsafe {
			let res = $crate::printk::DMESG.write_fmt(core::format_args!($($arg)*));
			if let Err(_) = res {
				panic!("printk failed");
			}
		}
	};
}
