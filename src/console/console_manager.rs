use crate::console::Console;
use crate::input::key_event::{Code, Key, KeyState};
use crate::input::keyboard::KeyboardEvent;

use super::key_record::KeyRecord;

const CONSOLE_COUNTS: usize = 4;

pub struct ConsoleManager {
	foreground: usize,
	console: [Console; CONSOLE_COUNTS],
	key_record: KeyRecord,
}

impl ConsoleManager {
	pub fn new() -> Self {
		ConsoleManager {
			key_record: KeyRecord::new(),
			foreground: 1,
			console: [
				Console::new(0),
				Console::new(1),
				Console::new(2),
				Console::new(3),
			],
		}
	}

	pub fn get_console<'a>(&'a mut self, index: usize) -> &'a mut Console {
		&mut self.console[index]
	}

	pub fn update(&mut self, kbd_ev: KeyboardEvent) {
		let console = &mut self.console[self.foreground];

		if let (Key::Control(c), KeyState::Pressed) = (kbd_ev.key, kbd_ev.state) {
			match c {
				// 0xa1..=0xa8 => 여기 어떻게 안되나요?
				Code::Home
				| Code::ArrowUp
				| Code::PageUp
				| Code::ArrowLeft
				| Code::ArrowRight
				| Code::End
				| Code::ArrowDown
				| Code::PageDown => console.move_cursor(c),
				Code::Delete => console.delete_char(),
				Code::Backspace => console.delete_char_and_move_cursor_left(), // 이름 추천
				_ => {}
			}
		}

		self.record_key(&kbd_ev);
		self.evaluate_key(kbd_ev.ascii);

		self.console[self.foreground].draw();
	}

	fn record_key(&mut self, kbd_ev: &KeyboardEvent) {
		let is_pressed = kbd_ev.state == KeyState::Pressed;

		match kbd_ev.key {
			Key::Printable(c, _) => {
				self.key_record.printable = if is_pressed { c } else { Code::None }
			}
			Key::Modifier(c, _) => match c {
				Code::Control => self.key_record.control = is_pressed,
				Code::Alt => self.key_record.alt = is_pressed,
				_ => {}
			},
			_ => {}
		}
	}

	fn evaluate_key(&mut self, ascii: u8) {
		let console = &mut self.console[self.foreground];
		let printable = self.key_record.printable;
		let control = self.key_record.control;
		let alt = self.key_record.alt;

		if let Code::None = printable {
			return;
		}

		let num = printable as usize - Code::N0 as usize;
		if control && num >= 1 && num <= 3 {
			self.foreground = num;
		} else if alt {
			console.change_color(printable);
		} else {
			console.put_char(ascii);
		}
	}
}
