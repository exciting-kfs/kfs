use core::ptr;

#[repr(u8)]
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

pub struct TextVGA;

impl TextVGA {
	pub const WIDTH: isize = 80;
	pub const HEIGHT: isize = 25;
	const MMIO_ADDR: *mut u16 = 0xb8000 as *mut u16;

	pub fn putc(y: isize, x: isize, c: Char) {
		if x >= Self::WIDTH || y >= Self::HEIGHT {
			panic!();
		}
		unsafe {
			ptr::write_volatile(Self::MMIO_ADDR.offset(y * Self::WIDTH + x), c.0);
		}
	}

	pub fn fill_reds() {
		let red = Char::styled(Attr::new(false, Color::Red, false, Color::Red), b'\0');
		// for i in 0..(Self::WIDTH * Self::HEIGHT) {
			unsafe { ptr::write_volatile(Self::MMIO_ADDR.offset(0), red.0) };
		// }
	}
}
