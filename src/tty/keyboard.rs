use core::arch::asm;

// const ARROW_PRESS_LEFT: u8 = 0x4b;
// const ARROW_PRESS_TOP: u8 = 0x48;
// const ARROW_PRESS_RIGHT: u8 = 0x4d;
// const ARROW_PRESS_DOWN: u8 = 0x50;
// const ARROW_RELEASE_LEFT: u8 = 0xcb;
// const ARROW_RELEASE_TOP: u8 = 0xc8;
// const ARROW_RELEASE_RIGHT: u8 = 0xcd;
// const ARROW_RELEASE_DOWN: u8 = 0xd0;

#[derive(PartialEq)]
enum KeyboardState {
	IMMEDIATE,
	SPECIAL, // CTRL, CMD, ARROW
	CAPSLOCK,
}

pub struct KeyInput {
	pub code: u8,
	pub ctrl: bool,
	pub alt: bool,
	pub shift: bool,
	pub cmd: bool,
	pub capslock: bool,
}

pub struct Keyboard {
	state: KeyboardState,
	input: u8,
	ctrl: bool,
	l_alt: bool,
	l_cmd: bool,
	l_shift: bool,
	r_alt: bool,
	r_cmd: bool,
	r_shift: bool,
	capslock: bool,
}

impl Keyboard {
	pub fn new() -> Self {
		Keyboard {
			state: KeyboardState::IMMEDIATE,
			input: 0,
			ctrl: false,
			l_alt: false,
			l_cmd: false,
			l_shift: false,
			r_alt: false,
			r_cmd: false,
			r_shift: false,
			capslock: false,
		}
	}

	pub fn read(&mut self) {
		let c;
		if Keyboard::can_read() {
			c = Keyboard::read_code();

			match self.state {
				KeyboardState::IMMEDIATE => self.immediate_state(c),
				KeyboardState::SPECIAL => self.special_state(c),
				KeyboardState::CAPSLOCK => self.capslock_state(c),
			}
		}
	}

	pub fn get_key_input(&mut self) -> Option<KeyInput> {
		if self.input == 0 {
			return None;
		}

		let k = KeyInput {
			code: self.input,
			ctrl: self.ctrl,
			alt: self.l_alt | self.r_alt,
			shift: self.l_shift | self.r_shift,
			cmd: self.l_cmd | self.r_cmd,
			capslock: self.capslock,
		};

		// TODO delete later
		let mut x = 0;
		while x < 1000000 {
			x += 1;
		}

		Some(k)
	}

	// fn print_char(&self, c:u8) {
	// 	static mut x: i32 = 0xb8000;
	// 	unsafe {
	// 		asm!(
	// 			"add eax, 0x2f00",
	// 			"mov [ebx], eax",
	// 			in("al") c,
	// 			in("ebx") x
	// 		);
	// 		x += 2;
	// 	}
	// }

	fn can_read() -> bool {
		let mut eax: u32 = 0;
		unsafe {
			asm!(
				"in al, 0x64",		// read kbd-controller status reg
				inout("eax") eax
			)
		}
		eax & 0x01 == 1
	}

	fn read_code() -> u8 {
		let mut ax: u16 = 0;
		unsafe {
			asm!(
				"in al, 0x60",		// read kbd-controller data reg
				inout("ax") ax
			)
		}
		ax as u8
	}

	fn immediate_state(&mut self, c: u8) {
		let is_released = c & 0x80 == 0x80;
		let is_pressed = !is_released;
		match c {
			0x3a => self.state = KeyboardState::CAPSLOCK,
			0xe0 => self.state = KeyboardState::SPECIAL,
			0x38 | 0xb8 => self.l_alt = is_pressed,
			0x2a | 0xaa => self.l_shift = is_pressed,
			0x36 | 0xb6 => self.r_shift = is_pressed,
			c if is_released => {
				if c & 0x7f == self.input & 0x7f {
					self.input = 0; // release
				}
			}
			c => self.input = c, // press
		}
	}

	fn special_state(&mut self, c: u8) {
		let is_pressed = c & 0x80 != 0x80;
		match c {
			0x4b | 0x48 | 0x4d | 0x50 => self.input = c,
			0x5b | 0xdb => self.l_cmd = is_pressed,
			0x5c | 0xdc => self.r_cmd = is_pressed,
			0x38 | 0xb8 => self.r_alt = is_pressed,
			0x1d | 0x9d => self.ctrl = is_pressed,
			_ => self.init_state(), // error
		}
	}

	fn capslock_state(&mut self, c: u8) {
		if c == 0xba {
			self.capslock = !self.capslock;
		} else {
			self.init_state(); // error
		}
	}

	fn init_state(&mut self) {
		self.state = KeyboardState::IMMEDIATE;
		self.input = 0;
		self.ctrl = false;
		self.l_alt = false;
		self.l_cmd = false;
		self.l_shift = false;
		self.r_alt = false;
		self.r_cmd = false;
		self.r_shift = false;
		self.capslock = false;
	}
}
