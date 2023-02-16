use crate::{input::{
	key_event::{Code, Key, KeyState},
	keyboard::KeyboardEvent,
}, driver::vga::text_vga};

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
		ReadOnlyConsole {
			inner: Console::new(),
			w_cursor: Cursor::new(0, 0),
		}
	}

	pub fn write_buf(&mut self, buf: &[u8], len: usize) {
		for i in 0..len {
			if self.endl(buf[i]) {
				continue;
			}
			self.inner.put_char_cursor(buf[i], self.w_cursor);
			self.w_cursor.x += 1;
		}
	}

	fn endl(&mut self, b: u8) -> bool {
		if b == b'\n' || (self.w_cursor.x >= BUFFER_WIDTH) {
			self.w_cursor.x = 0;
			self.w_cursor.y += 1;
			if self.w_cursor.y >= text_vga::HEIGHT {
				for _ in 0..text_vga::WIDTH {
					self.inner.buf.push(text_vga::Char::styled(self.inner.attr, b'\0'));
				}
				self.w_cursor.y = text_vga::HEIGHT - 1;
				self.inner.adjust_window_start(1);
			}
			return b == b'\n';
		}
		false
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
		} else if record.alt {
			self.inner.change_color(record.printable);
		}
	}
}
