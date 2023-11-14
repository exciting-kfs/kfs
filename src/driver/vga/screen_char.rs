use super::Attr;

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct Char(pub u16);

impl Char {
	pub const fn styled(attr: Attr, ch: u8) -> Self {
		Char(((attr.0 as u16) << 8) | (ch as u16))
	}

	pub const fn new(ch: u8) -> Self {
		Self::styled(Attr::default(), ch)
	}

	pub fn empty() -> Self {
		Self::styled(Attr::default(), b' ')
	}

	pub fn into_u8(self) -> u8 {
		(self.0 & (u8::MAX as u16)) as u8
	}

	pub fn get_attr(self) -> Attr {
		Attr((self.0 >> 8) as u8)
	}
}

impl Default for Char {
	fn default() -> Self {
		Self::empty()
	}
}

impl From<Char> for u8 {
	fn from(value: Char) -> Self {
		value.into_u8()
	}
}
