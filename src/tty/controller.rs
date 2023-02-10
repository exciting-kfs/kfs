// use super::screen::Screen;
use super::{keyboard::KeyInput, tty::Tty, tty::CODE_TO_ASCII};

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
		let code = key_input.code;
		if key_input.ctrl && TtyController::is_tty_index(code) {
			self.foreground = (CODE_TO_ASCII[code as usize] as u8 - '0' as u8) as usize;
			self.tty[self.foreground].draw();
		} else if key_input.alt {
			self.tty[self.foreground].set_attribute(key_input.code);
		} else {
			self.tty[self.foreground].input(key_input);
		}
	}

	fn is_tty_index(code: u8) -> bool {
		let code = CODE_TO_ASCII[code as usize];
		code >= '1' && code < '4'
	}
}
