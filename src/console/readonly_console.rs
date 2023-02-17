use crate::printkln;

use crate::{
	driver::vga::text_vga::{self, Char as VGAChar},
	input::{
		key_event::{Code, Key, KeyState},
		keyboard::KeyboardEvent,
	},
};

use super::{
	console::{Console, IConsole, BUFFER_HEIGHT, BUFFER_WIDTH},
	cursor::Cursor,
	key_record::KeyRecord,
};

pub struct ReadOnlyConsole {
	inner: Console,
	w_cursor: Cursor,
}

impl ReadOnlyConsole {
	pub fn new() -> Self {
		let mut inner = Console::new();

		let size = inner.buf.size();
		inner.buf.extend(BUFFER_HEIGHT * BUFFER_WIDTH - size);

		ReadOnlyConsole {
			inner,
			w_cursor: Cursor::new(0, 0),
		}
	}

	pub fn write_buf(&mut self, buf: &[u8]) {
		for ch in buf {
			let ch = *ch;

			if ch == b'\n' {
				self.endl();
				continue;
			}

			if self.w_cursor.x >= BUFFER_WIDTH {
				self.endl();
			}

			let ch = VGAChar::styled(self.inner.attr, ch);

			*self
				.inner
				.buf
				.at_mut(self.w_cursor.y * BUFFER_WIDTH + self.w_cursor.x)
				.unwrap() = ch;

			self.w_cursor.x += 1;
		}

		self.inner.window_start = usize::checked_sub(
			text_vga::WIDTH * (self.w_cursor.y + 1),
			text_vga::WIDTH * text_vga::HEIGHT,
		)
		.unwrap_or_default();
	}

	pub fn endl(&mut self) {
		self.w_cursor.y += 1;
		self.w_cursor.x = 0;

		if self.w_cursor.y >= BUFFER_HEIGHT {
			self.w_cursor.y -= 1;
			for _ in 0..BUFFER_WIDTH {
				self.inner.buf.push(VGAChar::styled(self.inner.attr, b'\0'));
			}
		}
	}
}

impl IConsole for ReadOnlyConsole {
	fn draw(&self) {
		self.inner.draw();
	}

	fn update(&mut self, ev: &KeyboardEvent, record: &KeyRecord) {
		if let (Key::Control(c), KeyState::Pressed) = (ev.key, ev.state) {
			match c {
				Code::Home
				| Code::ArrowUp
				| Code::PageUp
				| Code::ArrowLeft
				| Code::ArrowRight
				| Code::End
				| Code::ArrowDown
				| Code::PageDown => self.inner.move_cursor(c),
				_ => {}
			}
		}

		if let Code::None = record.printable {
			return;
		}

		if record.alt {
			self.inner.change_color(record.printable);
		}
	}
}
