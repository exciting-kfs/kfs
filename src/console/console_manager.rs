pub mod console;
mod cursor;
mod key_record;
mod readonly_console;

use console::{Console, IConsole};
use readonly_console::ReadOnlyConsole;

use super::ascii;

use crate::input::key_event::{KeyEvent, KeyKind};
use crate::input::keyboard::Keyboard;
use crate::text_vga::WINDOW_SIZE;
use crate::util::LazyInit;

use core::array;

pub static mut CONSOLE_MANAGER: LazyInit<ConsoleManager> = LazyInit::new(ConsoleManager::new);

const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	foreground: usize,
	read_only_on: bool,
	read_only: ReadOnlyConsole,
	keyboard: Keyboard,
	console: [Console; CONSOLE_COUNTS],
}

impl ConsoleManager {
	pub fn new() -> Self {
		ConsoleManager {
			foreground: 1,
			read_only_on: false,
			read_only: ReadOnlyConsole::new(),
			keyboard: Keyboard::new(),
			console: array::from_fn(|_| Console::buffer_reserved(WINDOW_SIZE)),
		}
	}

	pub fn update(&mut self, ev: KeyEvent) {
		self.keyboard.change_key_state(ev);

		if !ev.pressed() {
			return;
		}

		if let KeyKind::Function(v) = ev.identify() {
			let idx = (v.index() + 1) as usize;

			if idx <= CONSOLE_COUNTS {
				self.set_foreground(idx);
				return;
			}
		}

		let ascii = match ascii::convert(ev.key, &self.keyboard) {
			Some(c) => c,
			None => return,
		};

		if self.read_only_on {
			self.read_only.update(ascii);
		} else {
			let console = &mut self.console[self.foreground];
			console.update(ascii);
		}
	}

	pub fn draw(&self) {
		if self.read_only_on {
			self.read_only.draw();
		} else {
			let console = &self.console[self.foreground];
			console.draw();
		}
	}

	pub fn panic(&mut self, ev: KeyEvent) {
		// self.read_only.update(ev);
		// self.read_only.draw();
	}

	pub fn dmesg(&mut self) -> &mut ReadOnlyConsole {
		&mut self.read_only
	}

	fn set_foreground(&mut self, idx: usize) {
		self.foreground = idx;
		self.read_only_on = idx == CONSOLE_COUNTS;
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
