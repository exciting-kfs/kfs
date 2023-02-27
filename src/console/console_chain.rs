use super::console_manager::console::Console;

use crate::driver::tty::TTY;
use crate::driver::vga::text_vga::WINDOW_SIZE;
use crate::input::key_event::Code;
use crate::io::character::RW;

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

	// key -> tty -> console
	//            ->  sub -> console
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

	pub fn flush(&mut self) {
		self.flush_subroutine();
		self.flush_tty();
	}

	pub fn update(&mut self, code: Code) {
		self.tty.write(code);
		self.flush_tty();
	}

	pub fn draw(&self) {
		self.console.draw();
	}
}
