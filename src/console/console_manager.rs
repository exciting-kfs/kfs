//! Manage various consoles.

pub mod console;
pub mod cursor;

use core::array;

use super::ascii;
use super::console_chain::ConsoleChain;

use crate::input::key_event::{KeyEvent, KeyKind};
use crate::io::character::RW;
use crate::subroutine::{DMESG, RAW, SHELL};
use crate::util::LazyInit;

pub static mut CONSOLE_MANAGER: LazyInit<ConsoleManager> = LazyInit::new(ConsoleManager::new);

pub const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	foreground: usize,
	cons: [ConsoleChain; CONSOLE_COUNTS],
}

impl ConsoleManager {
	/// Create new manager with pre-defined work consoles.
	///
	/// you can switch between consoles with F1 ~ F4 key.
	///
	/// console
	/// 	- 0 => simple shell
	/// 	- 1 => raw console (echo on)
	///     - 2 => raw console (echo off)
	/// 	- 3 => kernel message buffer
	pub fn new() -> Self {
		ConsoleManager {
			foreground: 0,
			cons: array::from_fn(|i| {
				let (sub, echo) = unsafe {
					match i {
						0 => (&mut SHELL as &mut dyn RW<u8, u8>, false),
						1 => (&mut RAW[0] as &mut dyn RW<u8, u8>, true),
						2 => (&mut RAW[1] as &mut dyn RW<u8, u8>, false),
						3 => (&mut DMESG as &mut dyn RW<u8, u8>, false),
						_ => unreachable!("mismatch console count"),
					}
				};
				ConsoleChain::new(sub, echo)
			}),
		}
	}

	/// update console with new key event.
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

	/// change foreground console.
	pub fn set_foreground(&mut self, idx: usize) {
		if idx < CONSOLE_COUNTS {
			self.foreground = idx;
		}
	}

	pub fn flush_all(&mut self) {
		for console in &mut self.cons[..] {
			console.flush();
		}
	}
}
