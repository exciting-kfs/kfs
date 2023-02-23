//! Implements common keyboard tasks
//!
//! ### some example of common tasks
//!  - save pressed key state
//!  - key repeat rate / threshold

use super::key_event::{Code, KeyEvent};
use crate::driver::ps2::keyboard::get_key_event;

pub static mut KEYBOARD: Keyboard = Keyboard::new();

#[derive(Default)]
pub struct Keyboard {
	state: [u32; 8], // 256bit
}

impl Keyboard {
	pub const fn new() -> Self {
		Self::default() // false, false, false ...
	}

	/// 키보드에서 키 하나를 입력받고, 상태를 저장한 후, 받은 키를 반환한다.
	///
	/// # Returns
	///  - `None` -> 현재 키보드 버퍼에서 읽을 키가 존재하지 않음.
	///  - `Some(x)` -> 읽은 키에 대한 정보
	pub fn get_keyboard_event(&mut self) -> Option<KeyEvent> {
		let event = get_key_event()?;

		self.change_key_state(event);

		Some(KeyEvent {
			state: event.state,
			key: event.key,
		})
	}

	/// wait until key is pressed, then return received event.
	pub fn wait_keyboard_event(&mut self) -> KeyEvent {
		loop {
			if let Some(ev) = self.get_keyboard_event() {
				return ev;
			}
		}
	}

	pub fn pressed(&self, code: Code) -> bool {
		let (arr, bit) = Self::bit_index(code as u8);

		(self.state[arr] & (1 << bit)) != 0
	}

	fn bit_index(idx: u8) -> (usize, usize) {
		(idx as usize / 32, idx as usize % 32)
	}

	fn clear_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.state[arr] &= !(1 << bit);
	}

	fn set_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.state[arr] |= 1 << bit;
	}

	fn toggle_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.state[arr] ^= 1 << bit;
	}

	fn change_key_state(&mut self, event: KeyEvent) {
		let code = event.key.as_code();

		// pause doesn't have press / release state.
		if let Code::Pause = code {
			return;
		}

		if let Key::Toggle(..) = event.key {
			self.toggle_state_at(code as u8);
		} else {
			match event.state {
				KeyState::Pressed => self.set_state_at(code as u8),
				KeyState::Released => self.clear_state_at(code as u8),
			}
		}
	}
}
