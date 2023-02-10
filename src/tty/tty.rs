use super::keyboard::KeyInput;
use super::position::Position;
use super::screen::{IScreen, Screen, SCREEN_HEIGHT, SCREEN_WITDH};
// use super::screen::Screen;

pub const BUFFER_HEIGHT: usize = 100;
const BUFFER_WIDTH: usize = 80;
const SCREEN_POS_MAX: usize = BUFFER_HEIGHT - SCREEN_HEIGHT;

#[rustfmt::skip]
static CODE_TO_ASCII: [char; 128] = [
	'\0', '\0', '1', '2', '3', '4',  '5',  '6',  '7', '8',  '9',  '0',  '-',  '=', '\0', '\0', // null, ?, backspace, tab
	 'q',  'w', 'e', 'r', 't', 'y',  'u',  'i',  'o', 'p',  '[',  ']', '\n', '\0',  'a',  's',
	 'd',  'f', 'g', 'h', 'j', 'k',  'l',  ';', '\'', '`', '\0', '\\',  'z',  'x',  'c',  'v',
	 'b',  'n', 'm', ',', '.', '/', '\0', '\0', '\0', ' ', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

#[derive(Clone, Copy)]
pub struct Tty {
	frame_buffer: [[char; BUFFER_WIDTH]; BUFFER_HEIGHT],
	screen_pos: usize, // top
	cursor: Position,
	attribute: u8,
}

impl Tty {
	pub fn new() -> Self {
		Tty {
			frame_buffer: [['\0'; BUFFER_WIDTH]; BUFFER_HEIGHT],
			screen_pos: 0,
			cursor: Position(0, 0),
			attribute: 0x2f, // FIXME
		}
	}

	pub fn input(&mut self, key_input: KeyInput) {
		match key_input.code {
			0x4b => self.move_cursor(0, -1),
			0x48 => self.move_cursor(-1, 0),
			0x4d => self.move_cursor(0, 1),
			0x50 => self.move_cursor(1, 0),
			code => {
				let c = CODE_TO_ASCII[code as usize];
				let x = self.screen_pos + self.cursor.0 as usize;
				let y = self.cursor.1 as usize;
				Screen::putc(self.cursor, c, self.attribute);
				self.frame_buffer[x][y] = c;
				self.move_cursor(0, 1);
			}
		}
		self.draw();
	}

	pub fn set_attribute(&mut self, attribute: u8) {
		self.attribute = attribute;
	}

	pub fn draw(&mut self) {
		Screen::draw(&self.frame_buffer, self.screen_pos, self.attribute);
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
