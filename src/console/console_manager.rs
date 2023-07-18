//! Manage various consoles.

pub mod console;
pub mod cursor;

use core::mem::MaybeUninit;

use alloc::sync::Arc;
use alloc::vec::Vec;
use kfs_macro::context;

use self::console::{Console, SyncConsole};

use super::ascii;

use crate::config::CONSOLE_COUNTS;
use crate::driver::tty::{SyncTTY, TTY};
use crate::driver::vga::text_vga::WINDOW_SIZE;
use crate::input::key_event::{Code, KeyEvent, KeyKind};
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
			ttys.push(Arc::new(Locked::new(TTY::new(cons[i].clone(), true, true))));
		}

		ConsoleManager {
			foreground: Locked::new(0),
			cons,
			ttys,
		}
	}

	pub fn update(&self, code: Code) {
		let foreground = *self.foreground.lock();
		let _ = self.ttys[foreground].lock().write_one(code);
	}

	#[context(irq_disabled)]
	pub fn screen_draw(&self) {
		let foreground = *self.foreground.lock();
		self.cons[foreground].lock().draw();
	}

	/// change foreground console.
	#[context(irq_disabled)]
	pub fn set_foreground(&mut self, idx: usize) {
		if idx < CONSOLE_COUNTS {
			*self.foreground.lock() = idx;
		}
	}

	pub fn get_tty(&self, id: usize) -> SyncTTY {
		self.ttys[id].clone()
	}
}

pub fn console_manager_work(key_event: &mut KeyEvent) {
	unsafe {
		let cm = CONSOLE_MANAGER.assume_init_mut();

		if let KeyKind::Function(v) = key_event.identify() {
			let idx = v.index() as usize;

			if idx < CONSOLE_COUNTS {
				cm.set_foreground(idx);
			}
		}

		cm.screen_draw();
	}
}

pub fn init() {
	unsafe { CONSOLE_MANAGER.write(ConsoleManager::new()) };
}
