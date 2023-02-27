pub mod console;
pub mod cursor;

use core::array;

use super::ascii;
use super::console_chain::ConsoleChain;

use crate::input::key_event::{KeyEvent, KeyKind};
use crate::io::character::RW;
use crate::subroutine::dmesg::DMESG;
use crate::subroutine::shell::SHELL;
use crate::util::LazyInit;

pub static mut CONSOLE_MANAGER: LazyInit<ConsoleManager> = LazyInit::new(ConsoleManager::new);

pub const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	foreground: usize,
	cons: [ConsoleChain; CONSOLE_COUNTS],
}

impl ConsoleManager {
	pub fn new() -> Self {
		ConsoleManager {
			foreground: 0,
			cons: array::from_fn(|i| {
				let (task, echo) = if (i + 1) < CONSOLE_COUNTS {
					(unsafe { &mut SHELL[i] as &mut dyn RW<u8, u8> }, true)
				} else {
					(unsafe { &mut DMESG as &mut dyn RW<u8, u8> }, false)
				};

				ConsoleChain::new(task, echo)
			}),
		}
	}

	pub fn update(&mut self, ev: KeyEvent) {
		if !ev.pressed() {
			return;
		}

		if let KeyKind::Function(v) = ev.identify() {
			let idx = v.index() as usize;

			if idx < CONSOLE_COUNTS {
				self.foreground = idx;
				return;
			}
		}

		self.cons[self.foreground].update(ev.key);
	}

	pub fn draw(&self) {
		self.cons[self.foreground].draw();
	}

	pub fn set_foreground(&mut self, idx: usize) {
		if idx < CONSOLE_COUNTS {
			self.foreground = idx;
		}
	}

	pub fn dmesg(&mut self) -> &mut ConsoleChain {
		&mut self.cons[CONSOLE_COUNTS - 1]
	}
}

use core::fmt;

impl fmt::Write for ConsoleManager {
	fn write_str(&mut self, s: &str) -> fmt::Result {
		let dmesg = self.dmesg();

		for byte in s.as_bytes() {
			unsafe { DMESG.write(*byte) }
			dmesg.flush();
		}

		self.draw();
		Ok(())
	}
}
