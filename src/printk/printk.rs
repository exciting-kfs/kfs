use core::fmt;

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
