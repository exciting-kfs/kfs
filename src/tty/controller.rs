use super::{keyboard::KeyInput, tty::Tty};

const TTY_COUNTS: usize = 4;

pub struct TtyController {
	foreground: usize,
	tty: [Tty; TTY_COUNTS],
}

impl TtyController {
	pub fn new() -> Self {
		TtyController {
			foreground: 1,
			tty: [Tty::new(0), Tty::new(1), Tty::new(2), Tty::new(3)],
		}
	}

	pub fn get_tty<'a>(&'a mut self) -> &'a mut Tty {
		&mut self.tty[self.foreground]
	}

	pub fn input(&mut self, key_input: KeyInput) {
		let num = code_to_num(key_input.code);
		if key_input.ctrl && TtyController::is_tty_index(num) {
			self.foreground = num.unwrap() as usize;
		} else if key_input.alt {
			self.tty[self.foreground].set_attribute(key_input.code);
		} else {
			self.tty[self.foreground].input(key_input);
		}
		self.tty[self.foreground].draw();
	}

	fn is_tty_index(num: Option<u8>) -> bool {
		if let Some(n) = num {
			n >= 1 && n <= 3
		} else {
			false
		}
	}
}

// library?
fn code_to_num(code: u8) -> Option<u8> {
	match code {
		c @ 0x02..0x0a => Some(c - 1),
		0x0b => Some(0),
		_ => None,
	}
}
