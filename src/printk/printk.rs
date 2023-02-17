use crate::console::ConsoleManager;
use core::fmt;

impl fmt::Write for ConsoleManager {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		unsafe { self.dmesg().write_buf(s.as_bytes()) }
		Ok(())
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		let buf = [c as u8];
		unsafe { self.dmesg().write_buf(&buf) }
		Ok(())
	}
}

#[macro_export]
macro_rules! printkln {
	($($arg:tt)*) => {
		use core::fmt::Write;
		unsafe {
			let (res1, res2) = (
				$crate::console::CONSOLE_MANAGER.get().write_fmt(core::format_args!($($arg)*)),
				$crate::console::CONSOLE_MANAGER.get().write_char(b'\n' as char),
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
			let res = $crate::console::CONSOLE_MANAGER.get().write_fmt(core::format_args!($($arg)*));
			if let Err(_) = res {
				panic!("printk failed");
			}
		}
	};
}
