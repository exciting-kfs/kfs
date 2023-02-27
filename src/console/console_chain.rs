use super::console_manager::console::{
	Console, IConsole, BUFFER_HEIGHT, BUFFER_SIZE, BUFFER_WIDTH,
};

use crate::driver::tty::TTY;
use crate::input::key_event::{Code, KeyEvent, KeyKind};
use crate::io::character::{Read, Write, RW};

pub struct ConsoleChain {
	console: Console,
	tty: TTY,
	subroutine: &'static mut dyn RW<u8, u8>,
}

impl ConsoleChain {
	pub fn new(subroutine: &'static mut dyn RW<u8, u8>, echo: bool) -> Self {
		Self {
			console: Console::buffer_reserved(BUFFER_SIZE),
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

// pub fn write_buf(&mut self, buf: &[u8]) {
// 	for ch in buf {
// 		let ch = *ch;

// 		if ch == b'\n' {
// 			self.endl();
// 			continue;
// 		}

// 		if self.w_pos.x >= BUFFER_WIDTH {
// 			self.endl();
// 		}

// 		self.console.put_char_absolute(ch, &self.w_pos);
// 		self.w_pos.x += 1;
// 	}

// 	self.console.sync_window_start(self.w_pos.y + 1)
// }

// pub fn endl(&mut self) {
// 	self.w_pos.y += 1;
// 	self.w_pos.x = 0;

// 	if self.w_pos.y >= BUFFER_HEIGHT {
// 		self.w_pos.y -= 1;
// 		self.console.put_empty_line();
// 	}
// }
