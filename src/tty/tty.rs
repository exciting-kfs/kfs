use super::controller::TtyControl;
use super::keyboard::KeyboardToken;
use super::position::Position;
use super::screen::{IScreen, Screen, SCREEN_HEIGHT, SCREEN_WITDH};
// use super::screen::Screen;

pub const BUFFER_HEIGHT: usize = 100;
const BUFFER_WIDTH: usize = 80;
const SCREEN_POS_MAX: usize = BUFFER_HEIGHT - SCREEN_HEIGHT;

#[derive(Clone, Copy)]
pub struct Tty {
	frame_buffer: [[u8; BUFFER_WIDTH]; BUFFER_HEIGHT],
	screen_pos: usize, // top
	cursor: Position,
	attribute: u8,
}

impl Tty {
	pub const fn new() -> Self {
		Tty {
			frame_buffer: [[0; BUFFER_WIDTH]; BUFFER_HEIGHT],
			screen_pos: 0,
			cursor: Position(0, 0),
			attribute: 0x2f, // FIXME
		}
	}

	pub fn input(&mut self, token: KeyboardToken) {
		match token {
			KeyboardToken::Control(tc) => match tc {
				TtyControl::ChangeColor => self.attribute += 0x1, // FIXME
				TtyControl::MoveCursor(dx, dy) => self.move_cursor(dx, dy),
				_ => {} // logic error
			},
			KeyboardToken::Input(c) => {
				Screen::putc(c, self.attribute, self.cursor);
				self.move_cursor(0, 1);
			}
		}
	}

	pub fn draw(&mut self) {
		Screen::draw(self.screen_pos, &self.frame_buffer, self.attribute);
	}

	fn move_cursor(&mut self, dx: i8, dy: i8) {
		let mut x = self.cursor.0 as i8 + dx;
		let mut y = self.cursor.1 as i8 + dy;

		if y >= SCREEN_WITDH as i8 {
			x += 1;
			y = 0;
		}

		if y < 0 {
			x -= 1;
			y = SCREEN_WITDH as i8 - 1;
		}

		if x < 0 {
			let pos = self.screen_pos as i8 + x;
			self.screen_pos = if pos < 0 { 0 } else { pos as usize };
			x = 0;
		}

		if x >= SCREEN_HEIGHT as i8 {
			let pos = self.screen_pos as i8 + (x - SCREEN_HEIGHT as i8 + 1);
			if pos > SCREEN_POS_MAX as i8 {
				self.screen_pos = SCREEN_POS_MAX;
			} else {
				self.screen_pos = pos as usize;
			}
			x = SCREEN_HEIGHT as i8 - 1;
		}
		self.cursor = Position(x as u8, y as u8);
		Screen::put_cursor(self.cursor);
	}
}
