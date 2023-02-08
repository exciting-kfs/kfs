use super::controller::TtyControl;
use core::arch::asm;

const ARROW_PRESS_LEFT: u8 = 0x4b;
const ARROW_PRESS_TOP: u8 = 0x48;
const ARROW_PRESS_RIGHT: u8 = 0x4d;
const ARROW_PRESS_DOWN: u8 = 0x50;
const ARROW_RELEASE_LEFT: u8 = 0xcb;
const ARROW_RELEASE_TOP: u8 = 0xc8;
const ARROW_RELEASE_RIGHT: u8 = 0xcd;
const ARROW_RELEASE_DOWN: u8 = 0xd0;

enum KeyCode {
	ArrowLeft = 0x4b,
	ArrowTop = 0x48,
	ArrowRight = 0x4d,
	ArrowDown = 0x50,
}

pub enum KeyboardToken {
	Control(TtyControl),
	Input(char),
}

pub struct Keyboard {
	keymap: [u8; 128],
}

impl Keyboard {
	pub fn new() -> Self {
		Keyboard { keymap: [0; 128] }
	}

	pub fn read(&mut self) {
		let c;
		if Keyboard::can_read() {
			c = Keyboard::read_code();
			let key_num = (c & 0x0f) as usize;
			if c & 0x80 == 0x80 {
				self.keymap[key_num] = 0;
			} else {
				self.keymap[key_num] = 1;
			}
		}
	}

	fn can_read() -> bool {
		let mut eax: u32 = 0;
		unsafe {
			asm!(
				"in al, 0x64",		// read kbd-controller status reg
				// "add ax , 0x2f20",	// res + sp
				// "mov [0xb8f9e], ax",	// put char on last of vga text
				inout("eax") eax,
				options(nostack)
			)
		}
		eax & 0x01 == 1
	}

	fn read_code() -> u8 {
		let mut ax: u16 = 0;
		unsafe {
			asm!(
				"in al, 0x60",		// read kbd-controller data reg
				// "add ax , 0x3000", // res + sp
				// "mov [0xb8002], ax",
				inout("ax") ax
			)
		}
		ax as u8
	}

	pub fn get_token(&mut self) -> Option<KeyboardToken> {
		None
	}
}
