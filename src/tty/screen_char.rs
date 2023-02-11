#[allow(dead_code)]
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum Color {
	Black = 0,
	Blue = 1,
	Green = 2,
	Cyan = 3,
	Red = 4,
	Magenta = 5,
	Brown = 6,
	LightGray = 7,
	DarkGray = 8,
	LightBlue = 9,
	LightGreen = 10,
	LightCyan = 11,
	LightRed = 12,
	Pink = 13,
	Yellow = 14,
	White = 15,
}

#[derive(Clone, Copy)]
pub struct ColorCode(u8);

impl ColorCode {
	pub fn from_u8(code: u8) -> ColorCode {
		ColorCode(code)
	}

	pub fn new(foreground: Color, background: Color) -> ColorCode {
		ColorCode((background as u8) << 4 | foreground as u8)
	}

	fn to_u8(&self) -> u8 {
		self.0
	}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ScreenChar {
	pub color: ColorCode,
	pub character: char,
}

impl ScreenChar {
	pub fn new(color: ColorCode, character: char) -> Self {
		ScreenChar { color, character }
	}

	pub fn to_u16(&self) -> u16 {
		(self.color.to_u8() as u16) << 8 | self.character as u16
	}
}
