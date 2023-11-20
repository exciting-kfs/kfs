pub mod ascii;

mod console;
mod cursor;
mod termios;
mod tty;

pub use termios::WinSize;
pub use tty::TTYFile;

use alloc::sync::Arc;
use core::{
	mem::MaybeUninit,
	sync::atomic::{AtomicBool, Ordering},
};

use crate::{
	config::NR_CONSOLES, scheduler::work::once::WorkOnce, sync::Locked, syscall::errno::Errno,
};

use tty::TTY;

use self::termios::Termios;

use super::vga::get_text_window_size;

static FOREGROUND_TTY: Locked<MaybeUninit<TTYFile>> = Locked::uninit();
static mut TTYS: [MaybeUninit<TTYFile>; NR_CONSOLES] = MaybeUninit::uninit_array();

pub fn init() {
	work_init();

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

static IS_TTY_DETEACHED: AtomicBool = AtomicBool::new(false);

pub fn sys_deteach_tty() -> Result<usize, Errno> {
	IS_TTY_DETEACHED.store(true, Ordering::Relaxed);

	Ok(0)
}

pub fn sys_attach_tty() -> Result<usize, Errno> {
	IS_TTY_DETEACHED.store(false, Ordering::Relaxed);

	Ok(0)
}

pub fn get_foreground_tty() -> Option<TTYFile> {
	if IS_TTY_DETEACHED.load(Ordering::Relaxed) {
		return None;
	}

	let foreground = FOREGROUND_TTY.lock();

	Some(unsafe { foreground.assume_init_ref() }.clone())
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

fn console_screen_draw() {
	if let Some(tty) = get_foreground_tty() {
		tty.lock_tty().draw();
	}
}

static mut CONSOLE_SCREEN_DRAW: MaybeUninit<Arc<WorkOnce>> = MaybeUninit::uninit();

pub fn get_screen_draw_work() -> Arc<WorkOnce> {
	unsafe { CONSOLE_SCREEN_DRAW.assume_init_ref().clone() }
}

fn work_init() {
	unsafe { CONSOLE_SCREEN_DRAW.write(Arc::new(WorkOnce::new(console_screen_draw))) };
}
