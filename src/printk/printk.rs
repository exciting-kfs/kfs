#[macro_export]
macro_rules! printkln {
	($($arg:tt)*) => {
		use core::fmt::Write;
		unsafe {
				$crate::console::CONSOLE_MANAGER.get().write_str("\x1b[u").unwrap();
				$crate::console::CONSOLE_MANAGER.get().write_fmt(core::format_args!($($arg)*)).unwrap();
				$crate::console::CONSOLE_MANAGER.get().write_str("\n\x1b[s").unwrap();
		}
	};
}

#[macro_export]
macro_rules! printk {
	($($arg:tt)*) => {
		use core::fmt::Write;
		unsafe {
			$crate::console::CONSOLE_MANAGER.get().write_str("\x1b[u").unwrap();
			$crate::console::CONSOLE_MANAGER.get().write_fmt(core::format_args!($($arg)*)).unwrap();
			$crate::console::CONSOLE_MANAGER.get().write_str("\x1b[s").unwrap();
		}
	};
}
