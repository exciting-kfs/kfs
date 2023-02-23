use crate::input::{key_event::KeyKind, keyboard::KeyboardEvent};

use super::{
	console::{Console, IConsole, BUFFER_HEIGHT, BUFFER_SIZE, BUFFER_WIDTH},
	cursor::Cursor,
};

pub struct ReadOnlyConsole {
	inner: Console,
	w_pos: Cursor,
}

impl ReadOnlyConsole {
	pub fn new() -> Self {
		let inner = Console::buffer_reserved(BUFFER_SIZE);

		ReadOnlyConsole {
			inner,
			w_pos: Cursor::new(0, 0),
		}
	}

	pub fn write_buf(&mut self, buf: &[u8]) {
		for ch in buf {
			let ch = *ch;

			if ch == b'\n' {
				self.endl();
				continue;
			}

			if self.w_pos.x >= BUFFER_WIDTH {
				self.endl();
			}

			self.inner.put_char_absolute(ch, &self.w_pos);
			self.w_pos.x += 1;
		}

		self.inner.sync_window_start(self.w_pos.y + 1)
	}

	pub fn endl(&mut self) {
		self.w_pos.y += 1;
		self.w_pos.x = 0;

		if self.w_pos.y >= BUFFER_HEIGHT {
			self.w_pos.y -= 1;
			self.inner.put_empty_line();
		}
	}
}

impl IConsole for ReadOnlyConsole {
	fn draw(&self) {
		self.inner.draw();
	}

	fn update(&mut self, ev: &KeyboardEvent) {
		if !ev.event.pressed() {
			return;
		}

		if let KeyKind::Cursor(c) = ev.event.identify() {
			self.inner.move_cursor(c);
		}
	}
}
