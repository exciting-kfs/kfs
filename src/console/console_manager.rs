pub mod console;
mod cursor;
mod key_record;
mod readonly_console;

use console::{Console, IConsole};
use key_record::KeyRecord;
use readonly_console::ReadOnlyConsole;

use crate::input::key_event::{Code, KeyEvent, KeyKind};
use crate::input::keyboard::KeyboardEvent;
use crate::text_vga::WINDOW_SIZE;
use crate::util::LazyInit;

use core::array;

pub static mut CONSOLE_MANAGER: LazyInit<ConsoleManager> = LazyInit::new(ConsoleManager::new);

const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	foreground: usize,
	read_only_on: bool,
	read_only: ReadOnlyConsole,
	console: [Console; CONSOLE_COUNTS],
}

impl ConsoleManager {
	pub fn new() -> Self {
		ConsoleManager {
			foreground: 1,
			read_only_on: false,
			read_only: ReadOnlyConsole::new(),
			console: array::from_fn(|_| Console::buffer_reserved(WINDOW_SIZE)),
		}
	}

	pub fn update(&mut self, kbd_ev: KeyboardEvent) {
		if self.read_only_on {
			self.read_only.update(&kbd_ev);
			self.read_only.draw();
		} else {
			let console = &mut self.console[self.foreground];
			console.update(&kbd_ev);
			console.draw();
		}
	}

	pub fn panic(&mut self, kbd_ev: KeyboardEvent) {
		self.read_only.update(&kbd_ev);
		self.read_only.draw();
	}

	pub fn dmesg(&mut self) -> &mut ReadOnlyConsole {
		&mut self.read_only
	}

	// fn select_console(&mut self) {
	// 	let printable = self.key_record.printable;
	// 	let control = self.key_record.control;

	// 	if let Code::None = printable {
	// 		return;
	// 	}

	// 	let num = self.is_console_index(printable);
	// 	if control && num <= CONSOLE_COUNTS - 1 {
	// 		self.read_only_on = false;
	// 		self.foreground = num as usize;
	// 		self.key_record.printable = Code::None;
	// 	} else if control && printable == Code::Minus {
	// 		self.read_only_on = true;
	// 		self.key_record.printable = Code::None;
	// 	}
	// }

	fn is_console_index(&self, code: Code) -> usize {
		let code = code as usize;
		let n0 = Code::N0 as usize;
		if code >= n0 {
			code - n0
		} else {
			CONSOLE_COUNTS
		}
	}
}

use core::fmt;

impl fmt::Write for ConsoleManager {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		self.dmesg().write_buf(s.as_bytes());
		Ok(())
	}

	fn write_char(&mut self, c: char) -> fmt::Result {
		let buf = [c as u8];
		self.dmesg().write_buf(&buf);
		Ok(())
	}
}
