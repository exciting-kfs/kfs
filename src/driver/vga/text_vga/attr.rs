use super::Color;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Attr(pub u8);

impl Attr {
	pub const fn new(blink: bool, bg: Color, bright: bool, fg: Color) -> Self {
		Attr(((blink as u8) << 7) | ((bg as u8) << 4) | ((bright as u8) << 3) | (fg as u8))
	}

	pub const fn default() -> Self {
		Self::new(false, Color::Black, false, Color::White)
	}

	pub fn toggle_blink(self) -> Self {
		Attr((self.0) ^ (1 << 7))
	}

	pub fn toggle_bright(self) -> Self {
		Attr((self.0) ^ (1 << 3))
	}

	pub fn form_u8(attr: u8) -> Self {
		Attr(attr)
	}
}
