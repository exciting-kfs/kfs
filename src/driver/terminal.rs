pub mod ascii;

mod console;
mod cursor;
mod termios;
mod tty;

pub use termios::WinSize;
pub use tty::TTYFile;

use alloc::sync::Arc;
use core::mem::MaybeUninit;

use crate::{config::NR_CONSOLES, scheduler::work, sync::Locked};

use tty::TTY;

use self::termios::Termios;

use super::vga::get_text_window_size;

static FOREGROUND_TTY: Locked<MaybeUninit<TTYFile>> = Locked::uninit();
static mut TTYS: [MaybeUninit<TTYFile>; NR_CONSOLES] = MaybeUninit::uninit_array();

pub fn init() {
	for tty in unsafe { &mut TTYS } {
		tty.write(TTYFile::new(Arc::new(Locked::new(TTY::new(
			Termios::SANE,
			get_text_window_size(),
		)))));
	}

	unsafe {
		FOREGROUND_TTY
			.lock()
			.write(TTYS[0].assume_init_ref().clone())
	};
}

pub fn get_tty(idx: usize) -> Option<TTYFile> {
	if unsafe { TTYS.len() } <= idx {
		return None;
	}

	Some(unsafe { TTYS[idx].assume_init_ref() }.clone())
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
