pub mod ascii;

mod console;
mod cursor;
mod tty;

pub use tty::TTYFile;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use crate::{config::NR_CONSOLES, scheduler::work, sync::locked::Locked};

use tty::{TTYFlag, TTY};

static FOREGROUND_TTY: Locked<MaybeUninit<TTYFile>> = Locked::uninit();
static mut TTYS: [MaybeUninit<TTYFile>; NR_CONSOLES] = MaybeUninit::uninit_array();

pub fn init() {
	for tty in unsafe { &mut TTYS } {
		tty.write(TTYFile::new(Arc::new(Locked::new(TTY::new(TTYFlag::SANE)))));
	}

	unsafe {
		FOREGROUND_TTY
			.lock()
			.write(TTYS[0].assume_init_ref().clone())
	};
}

pub fn get_foreground_tty() -> TTYFile {
	let foreground = FOREGROUND_TTY.lock();

	unsafe { foreground.assume_init_ref() }.clone()
}

pub fn set_foreground_tty(idx: usize) {
	if unsafe { TTYS.len() } <= idx {
		return;
	}

	let mut foreground = FOREGROUND_TTY.lock();

	unsafe {
		foreground.assume_init_drop();
		foreground.write(TTYS[idx].assume_init_ref().clone());
	}
}

pub fn console_screen_draw(_: &mut ()) -> Result<(), work::Error> {
	get_foreground_tty().lock_tty().draw();

	Ok(())
}

// pub unsafe fn init() {
// 	let tty_files = (&mut TTY_FILES).iter_mut();
// 	let consoles = (&CONSOLES).iter();

// 	for (tty_file, console) in tty_files.zip(consoles) {
// 		let console = console.assume_init_ref().clone();

// 		tty_file.write(TTYFile::new(Arc::new(Locked::new(TTY::new(
// 			console,
// 			TTYFlag::SANE,
// 		)))));
// 	}
// }

// pub mod console;
// pub mod cursor;

// use core::mem::MaybeUninit;

// use alloc::sync::Arc;
// use alloc::vec::Vec;

// use self::console::Console;

// use super::ascii;

// use crate::config::CONSOLE_COUNT;
// use crate::driver::tty::{TTYFile, TTYFlag, TTY};
// use crate::driver::vga::text_vga::WINDOW_SIZE;
// use crate::input::key_event::Code;
// use crate::io::ChWrite;
// use crate::scheduler::work::Error;
// use crate::sync::locked::Locked;

// pub static mut CONSOLE_MANAGER: MaybeUninit<ConsoleManager> = MaybeUninit::uninit();

// pub struct ConsoleManager {
// 	foreground: Locked<usize>,
// 	cons: Vec<Arc<Locked<Console>>>,
// 	ttys: Vec<TTYFile>,
// }

// impl ConsoleManager {
// 	/// Create new manager with pre-defined work consoles.
// 	///
// 	/// you can switch between consoles with F1 ~ F4 key.
// 	///
// 	/// console
// 	/// 	- 0 => simple shell
// 	/// 	- 1 => raw console (echo on)
// 	///     - 2 => raw console (echo off)
// 	/// 	- 3 => kernel message buffer
// 	pub fn new() -> Self {
// 		let mut cons = Vec::new();
// 		let mut ttys = Vec::new();
// 		cons.reserve(CONSOLE_COUNT);

// 		for _ in 0..CONSOLE_COUNT {
// 			cons.push(Arc::new(Locked::new(Console::buffer_reserved(WINDOW_SIZE))));
// 		}

// 		for i in 0..CONSOLE_COUNT {
// 			ttys.push(TTYFile::new(Arc::new(Locked::new(TTY::new(
// 				cons[i].clone(),
// 				TTYFlag::SANE,
// 			)))));
// 		}

// 		ConsoleManager {
// 			foreground: Locked::new(0),
// 			cons,
// 			ttys,
// 		}
// 	}

// 	pub fn update(&self, code: Code) {
// 		let foreground = self.foreground.lock();
// 		let mut tty = self.ttys[*foreground].lock_tty();
// 		let _ = tty.write_one(code);
// 	}

// 	pub fn screen_draw(&self) {
// 		let foreground = self.foreground.lock();
// 		let console = self.cons[*foreground].lock();
// 		console.draw();
// 	}

// 	/// change foreground console.
// 	pub fn set_foreground(&self, idx: usize) {
// 		if idx < CONSOLE_COUNT {
// 			let mut foreground = self.foreground.lock();
// 			*foreground = idx;
// 		}
// 	}

// 	pub fn get_tty(&self, id: usize) -> TTYFile {
// 		self.ttys[id].clone()
// 	}
// }

// pub fn console_screen_draw(_: &mut ()) -> Result<(), Error> {
// 	unsafe { CONSOLE_MANAGER.assume_init_mut().screen_draw() };
// 	Ok(())
// }

// pub fn init() {
// 	unsafe { CONSOLE_MANAGER.write(ConsoleManager::new()) };
// }
