//! Manage various consoles.

pub mod console;
pub mod cursor;

use core::mem::MaybeUninit;

use alloc::sync::Arc;
use alloc::vec::Vec;

use self::console::{Console, SyncConsole};

use super::ascii;

use crate::config::CONSOLE_COUNTS;
use crate::driver::tty::{SyncTTY, TTYFlag, TTY};
use crate::driver::vga::text_vga::WINDOW_SIZE;
use crate::input::key_event::Code;
use crate::io::ChWrite;
use crate::sync::locked::Locked;

pub static mut CONSOLE_MANAGER: MaybeUninit<ConsoleManager> = MaybeUninit::uninit();

pub struct ConsoleManager {
	foreground: Locked<usize>,
	cons: Vec<SyncConsole>,
	ttys: Vec<SyncTTY>,
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
		let mut cons = Vec::new();
		let mut ttys = Vec::new();
		cons.reserve(CONSOLE_COUNTS);

		for _ in 0..CONSOLE_COUNTS {
			cons.push(Arc::new(Locked::new(Console::buffer_reserved(WINDOW_SIZE))));
		}

		for i in 0..CONSOLE_COUNTS {
			ttys.push(Arc::new(Locked::new(TTY::new(
				cons[i].clone(),
				TTYFlag::SANE,
			))));
		}

		ConsoleManager {
			foreground: Locked::new(0),
			cons,
			ttys,
		}
	}

	pub fn update(&self, code: Code) {
		let foreground = self.foreground.lock();
		let mut tty = self.ttys[*foreground].lock();
		let _ = tty.write_one(code);
	}

	pub fn screen_draw(&self) {
		let foreground = self.foreground.lock();
		let console = self.cons[*foreground].lock();
		console.draw();
	}

	/// change foreground console.
	pub fn set_foreground(&self, idx: usize) {
		if idx < CONSOLE_COUNTS {
			let mut foreground = self.foreground.lock();
			*foreground = idx;
		}
	}

	pub fn get_tty(&self, id: usize) -> SyncTTY {
		self.ttys[id].clone()
	}
}

pub fn console_screen_draw(_: &mut ()) {
	unsafe { CONSOLE_MANAGER.assume_init_mut().screen_draw() };
}

pub fn init() {
	unsafe { CONSOLE_MANAGER.write(ConsoleManager::new()) };
}
