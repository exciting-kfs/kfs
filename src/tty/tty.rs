use super::keyboard::KeyInput;
use super::position::Position;
use super::screen::{IScreen, Screen, SCREEN_HEIGHT, SCREEN_WITDH};
use super::screen_char::{Color, ColorCode, ScreenChar};

pub const BUFFER_HEIGHT: usize = 100;
const BUFFER_WIDTH: usize = 80;

#[rustfmt::skip]
pub static CODE_TO_ASCII: [char; 128] = [
	'\0',  '\0',  '1',  '2',  '3',  '4',  '5',  '6',  '7',  '8',  '9',  '0',  '-',  '=', '\0', '\0', // null, ?, backspace, tab
	 'q',   'w',  'e',  'r',  't',  'y',  'u',  'i',  'o',  'p',  '[',  ']', '\n', '\0',  'a',  's',
	 'd',   'f',  'g',  'h',  'j',  'k',  'l',  ';', '\'',  '`', '\0', '\\',  'z',  'x',  'c',  'v',
	 'b',   'n',  'm',  ',',  '.',  '/', '\0', '\0', '\0',  ' ', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
	 '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0', '\0',
];

pub struct Tty {
	index: u8,
	buffer: [[ScreenChar; BUFFER_WIDTH]; BUFFER_HEIGHT],
	buffer_top: usize,
	screen_top: usize, // screen top
	cursor: Position,
	default_color: ColorCode,
}

impl Tty {
	pub fn new(index: u8) -> Self {
		let default_color = ColorCode::new(Color::White, Color::Green);
		let default_char = ScreenChar::new(default_color, '\0');
		Tty {
			index,
			buffer: [[default_char; BUFFER_WIDTH]; BUFFER_HEIGHT],
			buffer_top: 0,
			screen_top: 0,
			cursor: Position(0, 0),
			default_color,
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
				let s = ScreenChar::new(self.default_color, c);
				let x = self.screen_top + self.cursor.0 as usize;
				let y = self.cursor.1 as usize;
				Screen::putc(self.cursor, s);
				self.buffer[x][y] = s;
				self.move_cursor(0, 1);
			}
		}
	}

	pub fn set_default_color(&mut self, default_color: ColorCode) {
		self.default_color = default_color;
	}

	pub fn draw(&mut self) {
		Screen::draw(&self.buffer, self.screen_top);
		Screen::put_cursor(self.cursor);

		let right_bot = Position(SCREEN_HEIGHT as u8, SCREEN_WITDH as u8 - 1);
		let color = ColorCode::new(Color::White, Color::Black);
		let ch = ScreenChar::new(color, (0x30 + self.index as u8) as char);
		Screen::putc(right_bot, ch);
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
			let top = self.screen_top as isize + x as isize;
			self.screen_top = self.calc_screen_top(top);
			x = 0;
		}

		if x >= SCREEN_HEIGHT as i8 {
			let top = self.screen_top as isize + x as isize;
			let top = top - SCREEN_HEIGHT as isize + 1;
			self.screen_top = self.calc_screen_top(top);
			x = SCREEN_HEIGHT as i8 - 1;
		}
		self.cursor = Position(x as u8, y as u8);
		Screen::put_cursor(self.cursor);
	}

	fn get_screen_top_max(&self) -> usize {
		if self.buffer_top < SCREEN_HEIGHT {
			BUFFER_HEIGHT + self.buffer_top - SCREEN_HEIGHT
		} else {
			self.buffer_top - SCREEN_HEIGHT
		}
	}

	fn calc_screen_top(&mut self, top: isize) -> usize {
		let screen_top_max = self.get_screen_top_max();

		if top > screen_top_max as isize {
			screen_top_max
		} else if top < self.buffer_top as isize {
			self.buffer_top
		} else {
			top as usize
		}
	}
}
