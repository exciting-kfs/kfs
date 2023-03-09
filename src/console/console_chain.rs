//! Console related I/O helper

use super::console_manager::console::Console;

use crate::driver::tty::TTY;
use crate::driver::vga::text_vga::WINDOW_SIZE;
use crate::input::key_event::Code;
use crate::io::character::RW;

/// connects KEYBOARD - TTY - SUBROUTINE - CONSOLE - VGA
///
/// brief topology is
///```
/// KEYBOARD --> TTY ---(echo)---> CONSOLE --> VGA
///               `--> SUBROUTINE -->`
/// ```
pub struct ConsoleChain {
	console: Console,
	tty: TTY,
	subroutine: &'static mut dyn RW<u8, u8>,
}

impl ConsoleChain {
	pub fn new(subroutine: &'static mut dyn RW<u8, u8>, echo: bool) -> Self {
		Self {
			console: Console::buffer_reserved(WINDOW_SIZE),
			tty: TTY::new(echo),
			subroutine,
		}
	}

	/// Update device with new keyboard event.
	///
	/// We have to perform I/O in depth-first.
	/// since almost all related devices doesn't have their internal buffer.
	pub fn update(&mut self, code: Code) {
		self.flush();
		self.tty.write(code);
		self.flush_tty();
	}

	/// Flush all prepared but not performd I/O
	///
	/// In order to avoid data loss, do flush reverse way.
	///
	/// exact order is
	///
	/// 1) `subroutine` -> `console` (from `flush_subroutine()`)
	/// 2) `tty` -> `console` (from `flush_tty()`)
	/// 3) `tty` -> `subroutine` (from `flush_tty()`)
	/// 4) `subroutine` -> `console` (from `flush_subroutine()` in `flush_tty()`)
	pub fn flush(&mut self) {
		self.flush_subroutine();
		self.flush_tty();
	}

	/// draw console buffer to screen.
	pub fn draw(&self) {
		self.console.draw();
	}

	fn flush_tty(&mut self) {
		while let Some(ascii) = self.tty.read_echo() {
			self.console.write(ascii);
		}

		while let Some(ascii) = self.tty.read_task() {
			self.subroutine.write_one(ascii);
			self.flush_subroutine();
		}
	}

	fn flush_subroutine(&mut self) {
		while let Some(x) = self.subroutine.read_one() {
			self.console.write(x);
		}
	}
}
