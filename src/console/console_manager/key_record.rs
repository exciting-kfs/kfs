use crate::input::key_event::Code;

pub struct KeyRecord {
	pub control: bool,
	pub alt: bool,
	pub printable: Code,
}

impl KeyRecord {
	pub const fn new() -> Self {
		KeyRecord {
			control: false,
			alt: false,
			printable: Code::Unknown,
		}
	}
}
