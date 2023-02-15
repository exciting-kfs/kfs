use crate::input::key_event::{Code, Key, KeyState};
use crate::input::keyboard::KeyboardEvent;

use crate::printk::DMESG;

use super::console::{Console, IConsole};
use super::key_record::KeyRecord;
use super::readonly_console::ReadOnlyConsole;

pub static mut CONSOLE_MANAGER: ConsoleManager = ConsoleManager::new();

const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	key_record: KeyRecord,
	foreground: usize,
	read_only_on: bool,
	read_only: ReadOnlyConsole,
	console: [Console; CONSOLE_COUNTS],
}

impl ConsoleManager {
	pub const fn new() -> Self {
		ConsoleManager {
			key_record: KeyRecord::new(),
			foreground: 1,
			read_only_on: false,
			read_only: ReadOnlyConsole::new(),
			console: [Console::new(); 4],
		}
	}

	pub fn update(&mut self, kbd_ev: KeyboardEvent) {
		self.record_key(&kbd_ev);
		self.select_console();

		if self.read_only_on {
			self.read_only.update(&kbd_ev, &self.key_record);
			self.read_only.draw();
		} else {
			let console = &mut self.console[self.foreground];
			console.update(&kbd_ev, &self.key_record);
			console.draw();
		}
	}

	pub fn dmesg(&mut self) -> &mut ReadOnlyConsole {
		&mut self.read_only
	}

	fn record_key(&mut self, kbd_ev: &KeyboardEvent) {
		let is_pressed = kbd_ev.state == KeyState::Pressed;

		match kbd_ev.key {
			Key::Printable(c, _) => {
				self.key_record.printable = if is_pressed { c } else { Code::None }
			}
			Key::Modifier(c, _) => match c {
				Code::Control => self.key_record.control = is_pressed,
				Code::Alt => self.key_record.alt = is_pressed,
				_ => {}
			},
			_ => {}
		}
	}

	fn select_console(&mut self) {
		let printable = self.key_record.printable;
		let control = self.key_record.control;

		if let Code::None = printable {
			return;
		}

		let num = self.is_console_index(printable);
		if control && num <= CONSOLE_COUNTS - 1 {
			self.read_only_on = false;
			self.foreground = num as usize;
			self.key_record.printable = Code::None;
		} else if control && printable == Code::Minus {
			self.read_only_on = true;
			self.key_record.printable = Code::None;
			unsafe { DMESG.flush() };
		}
	}

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
