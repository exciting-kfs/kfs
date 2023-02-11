#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Color {
	Black = 0,
	Blue = 1,
	Green = 2,
	Cyan = 3,
	Red = 4,
	Magenta = 5,
	Brown = 6,
	White = 7,
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Attr(u8);

impl Attr {
	pub fn new(blink: bool, bg: Color, bright: bool, fg: Color) -> Self {
		Attr(((blink as u8) << 7) | ((bg as u8) << 4) | ((bright as u8) << 3) | (fg as u8))
	}

	pub fn default() -> Self {
		Self::new(false, Color::Black, false, Color::White)
	}

	pub fn toggle_blink(self) -> Self {
		Attr((self.0) ^ (1 << 7))
	}

	pub fn toggle_bright(self) -> Self {
		Attr((self.0) ^ (1 << 3))
	}
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Char(u16);

impl Char {
	pub fn styled(attr: Attr, ch: u8) -> Self {
		Char(((attr.0 as u16) << 8) | (ch as u16))
	}

	pub fn new(ch: u8) -> Self {
		Self::styled(Attr::default(), ch)
	}

	pub fn empty() -> Self {
		Self::styled(Attr::default(), b'\0')
	}
}

pub mod TextVGA {
	use super::*;
	use core::ptr;

	pub const WIDTH: usize = 80;
	pub const HEIGHT: usize = 25;
	const MMIO_ADDR: *mut u16 = 0xb8000 as *mut u16;

	pub fn putc(y: usize, x: usize, c: Char) {
		if x >= WIDTH || y >= HEIGHT {
			panic!("putc: invalid coordinate ({y}, {x}), ");
		}
		unsafe {
			ptr::write_volatile(MMIO_ADDR.offset((y * WIDTH + x) as isize), c.0);
		}
	}

	pub fn clear() {
		let black = Char::styled(Attr::new(false, Color::Black, false, Color::Black), b'\0');
		for i in 0..(HEIGHT) {
			for j in 0..(WIDTH) {
				putc(i, j, black)
			}
		}
	}
}
