// use super::screen::Screen;
use super::{keyboard::KeyboardToken, tty::Tty};

const TTY_COUNTS: usize = 4;

pub enum TtyControl {
	ChangeTty,
	// ShowDmesg,
	// CloseDmesg,
	ChangeColor,
	MoveCursor(i8, i8),
}

pub struct TtyController {
	foreground: usize,
	tty: [Tty; TTY_COUNTS],
}

impl TtyController {
	pub const fn new() -> Self {
		TtyController {
			foreground: 0,
			tty: [Tty::new(); 4],
		}
	}

	pub fn get_tty<'a>(&'a mut self) -> &'a mut Tty {
		&mut self.tty[self.foreground]
	}

	pub fn input(&mut self, token: KeyboardToken) {
		let foreground = self.foreground;
		let tty = &mut self.tty[foreground];
		match token {
			KeyboardToken::Control(ref tc) => match tc {
				TtyControl::ChangeTty => {
					let next = (foreground + 1) % 4;
					self.tty[next].draw();
					self.foreground = next;
				}
				_ => tty.input(token),
			},
			KeyboardToken::Input(_) => {
				if foreground != 0 {
					tty.input(token)
				}
			}
		}
	}
}
