// use super::screen::Screen;
use super::{keyboard::KeyInput, tty::Tty};

const TTY_COUNTS: usize = 4;

pub struct TtyController {
	foreground: usize,
	tty: [Tty; TTY_COUNTS],
}

impl TtyController {
	pub fn new() -> Self {
		TtyController {
			foreground: 0,
			tty: [Tty::new(); 4],
		}
	}

	pub fn get_tty<'a>(&'a mut self) -> &'a mut Tty {
		&mut self.tty[self.foreground]
	}

	pub fn input(&mut self, key_input: KeyInput) {
		if key_input.ctrl && TtyController::is_tty_index(key_input.code) {
			self.foreground = (key_input.code - '0' as u8) as usize;
			self.tty[self.foreground].draw();
		} else if key_input.alt {
			self.tty[self.foreground].set_attribute(key_input.code);
		} else {
			self.tty[self.foreground].input(key_input);
		}
	}

	fn is_tty_index(code: u8) -> bool {
		code >= b'1' && code < b'4'
	}
}
