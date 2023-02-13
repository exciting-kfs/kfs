use crate::driver::vga::text_vga::{self, Attr as VGAAttr, Char as VGAChar};
use crate::input::key_event::Code;

pub const BUFFER_HEIGHT: usize = 100;
const BUFFER_WIDTH: usize = 80;

struct Position {
	y: usize,
	x: usize,
}

impl Position {
	fn new(y: usize, x: usize) -> Self {
		Position { x, y }
	}
}

pub struct Console {
	id: u8,
	buf: [[text_vga::Char; BUFFER_WIDTH]; BUFFER_HEIGHT],
	buf_top: usize,
	vga_top: usize,
	cursor: Position,
	attr: VGAAttr,
}

impl Console {
	pub fn new(id: u8) -> Self {
		let default_char = VGAChar::new(0);
		Console {
			id,
			buf: [[default_char; BUFFER_WIDTH]; BUFFER_HEIGHT],
			buf_top: 0,
			vga_top: 0,
			cursor: Position::new(0, 0),
			attr: VGAAttr::default(),
		}
	}

	// pub fn input(&mut self, key_input: KeyInput) {
	// 	match key_input.code {
	// 		0x4b => self.move_cursor(0, -1),
	// 		0x48 => self.move_cursor(-1, 0),
	// 		0x4d => self.move_cursor(0, 1),
	// 		0x50 => self.move_cursor(1, 0),
	// 		code => {
	// 			let c = CODE_TO_ASCII[code as usize];
	// 			let s = text_vga::Char::new(self.default_color, c);
	// 			let x = self.vga_top + self.cursor.0 as usize;
	// 			let y = self.cursor.1 as usize;
	// 			Screen::putc(self.cursor, s);
	// 			self.buf[x][y] = s;
	// 			self.move_cursor(0, 1);
	// 		}
	// 	}
	// }

	pub fn put_char(&mut self, c: u8) {
		let ch = VGAChar::styled(self.attr, c);
		let y = self.cursor.y + self.vga_top;
		let x = self.cursor.x;

		self.buf[y][x] = ch;
		self._move_cursor(0, 1);
	}

	pub fn delete_char(&mut self) {
		let ch = VGAChar::styled(self.attr, 0);
		let y = self.cursor.y + self.vga_top;
		let x = self.cursor.x;

		self.buf[y][x] = ch;
	}

	pub fn delete_char_and_move_cursor_left(&mut self) {
		self._move_cursor(0, -1);

		let ch = VGAChar::styled(self.attr, 0);
		let y = self.cursor.y + self.vga_top;
		let x = self.cursor.x;

		self.buf[y][x] = ch;
	}

	pub fn change_color(&mut self, color: Code) {
		let color = color as u16;
		self.attr = VGAAttr::form_u8(color as u8);

		for y in 0..BUFFER_HEIGHT {
			for x in 0..BUFFER_WIDTH {
				let ch = self.buf[y][x];
				let ch = VGAChar(color << 8 | ch.0 & 0x00ff);
				self.buf[y][x] = ch;
			}
		}
	}

	pub fn move_cursor(&mut self, code: Code) {
		let home = -(self.cursor.x as isize);
		let end = (BUFFER_WIDTH - self.cursor.x - 1) as isize;
		let up = -(text_vga::HEIGHT as isize) + 1;
		let down = text_vga::HEIGHT as isize - 1;

		match code {
			Code::Home => self._move_cursor(0, home),
			Code::ArrowUp => self._move_cursor(-1, 0),
			Code::PageUp => self._move_cursor(up, 0),
			Code::ArrowLeft => self._move_cursor(0, -1),
			Code::ArrowRight => self._move_cursor(0, 1),
			Code::End => self._move_cursor(0, end),
			Code::ArrowDown => self._move_cursor(1, 0),
			Code::PageDown => self._move_cursor(down, 0),
			_ => {}
		}
	}

	pub fn draw(&mut self) {
		text_vga::draw(&self.buf, self.vga_top);
		text_vga::put_cursor(self.cursor.y, self.cursor.x);
	}

	fn _move_cursor(&mut self, dy: isize, dx: isize) {
		let mut y = self.cursor.y as isize + dy;
		let mut x = self.cursor.x as isize + dx;
		let vga_width: isize = text_vga::WIDTH as isize;
		let vga_height: isize = text_vga::HEIGHT as isize;

		if x >= vga_width {
			y += 1;
			x = 0;
		}

		if x < 0 {
			y -= 1;
			x = vga_width - 1;
		}

		if y < 0 {
			let top = self.vga_top as isize + y;
			self.vga_top = self.calc_vga_top(top);
			y = 0;
		}

		if y >= vga_height {
			let top = self.vga_top as isize + y;
			let top = top - vga_height + 1;
			self.vga_top = self.calc_vga_top(top);
			y = vga_height - 1;
		}

		self.cursor = Position::new(y as usize, x as usize);
		text_vga::put_cursor(self.cursor.y, self.cursor.x);
	}

	fn get_vga_top_max(&self) -> usize {
		if self.buf_top < text_vga::HEIGHT {
			BUFFER_HEIGHT + self.buf_top - text_vga::HEIGHT
		} else {
			self.buf_top - text_vga::HEIGHT
		}
	}

	fn calc_vga_top(&mut self, top: isize) -> usize {
		let vga_top_max = self.get_vga_top_max();

		if top > vga_top_max as isize {
			vga_top_max
		} else if top < self.buf_top as isize {
			self.buf_top
		} else {
			top as usize
		}
	}
}
