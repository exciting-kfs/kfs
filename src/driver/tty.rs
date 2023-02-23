use crate::input::key_event::{Code, Key, KeyEvent, PrintVar};
use crate::input::keyboard::KEYBOARD;

use crate::collection::WrapQueue;

const TTY_BUFFER_SIZE: usize = 1024;

const ASCII_MAX: u8 = 127;

const fn esc(a: u8, b: u8) -> [u8; 4] {
    [b'\x1b', b'\x5b', a, b]
}

const fn func(n: u8) -> [u8; 3] {
    [b'\x1b', b'O', n]
}


const DEL: 

// TODO: more settings - termios
struct TTY {
	buf: WrapQueue<u8, TTY_BUFFER_SIZE>,
}

impl TTY {
	pub fn new() -> Self {
		TTY {
			buf: WrapQueue::from_fn(|_| Default::default()),
		}
	}

	pub fn put(&mut self, event: KeyEvent) {
		// buffer is full.
		if self.buf.full() {
			return;
		}

		if let Key::Printable(code, var) = event.key {
			self.buf.push(Self::printable_to_ascii(code as u8, var));
			return;
		}

		let code = event.key.as_code();
		if (code as u8) <= ASCII_MAX {
			self.buf.push(code as u8);
			return;
		}

		match code {}
	}

	pub fn get(&mut self) -> Option<u8> {
		self.buf.pop()
	}

	fn puts(&mut self, s: &[u8]) {
		if self.buf.size() < s.len() {
			return;
		}

		for c in s {
			self.buf.push(*c);
		}
	}

	fn shift() -> bool {
		unsafe { KEYBOARD.pressed(Code::Capslock) || KEYBOARD.pressed(Code::Shift) }
	}

	/// 알파벳 대소문자 처리
	fn alpha_to_ascii(code: u8) -> u8 {
		if Self::shift() {
			code.to_ascii_uppercase()
		} else {
			code
		}
	}

	/// 숫자 / 특수문자 변환 처리
	/// 키가 넘패드에서 눌린 것이 아닐 때만 변환.
	fn numpad_to_ascii(code: u8, var: PrintVar) -> u8 {
		let need_shift = Self::shift() && var == PrintVar::Regular;

		if !need_shift {
			return code;
		}

		match code {
			b'1' => b'!',
			b'2' => b'@',
			b'3' => b'#',
			b'4' => b'$',
			b'5' => b'%',
			b'6' => b'^',
			b'7' => b'&',
			b'8' => b'*',
			b'9' => b'(',
			b'0' => b')',
			b'-' => b'_',
			b'/' => b'?',
			b'.' => b'>',
			_ => unreachable!("code must be exist on both regular / numpad"),
		}
	}

	/// 추가적으로 shift를 눌렀을 때 변화가 일어나야 하는 키들의 처리
	fn others_to_ascii(code: u8) -> u8 {
		if !Self::shift() {
			return code;
		}

		match code {
			b'`' => b'~',
			b'=' => b'+',
			b'[' => b'{',
			b']' => b'}',
			b'\\' => b'|',
			b';' => b':',
			b'\'' => b'"',
			b',' => b'>',
			_ => unreachable!("unknown code for others"),
		}
	}

	/// shift / capslock 등 현재 키 입력 상태에 따라 다른 ascii 표현을 가지는 키를 처리
	fn printable_to_ascii(code: u8, var: PrintVar) -> u8 {
		match code {
			b'a'..=b'z' => Self::alpha_to_ascii(code),
			b'0'..=b'9' | b'-' | b'/' | b'.' => Self::numpad_to_ascii(code, var),
			b'`' | b'=' | b'[' | b']' | b'\\' | b';' | b'\'' | b',' => Self::others_to_ascii(code),
			_ => unreachable!("unknown code detected"),
		}
	}
}
