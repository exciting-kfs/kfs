//! Implements common keyboard tasks
//!
//! ### some example of common tasks
//!  - save pressed key state
//!  - key repeat rate / threshold

use alloc::sync::Arc;

use super::key_event::{Code, KeyEvent, KeyKind};
use crate::syscall::errno::Errno;

pub static mut KEYBOARD: Keyboard = Keyboard::new();

pub trait KbdDriver {
	fn get_key_event(&self) -> Option<KeyEvent>;
	fn reset_cpu(&self);
}

#[derive(Default)]
pub struct Keyboard {
	driver: Option<Arc<dyn KbdDriver>>,
	pressed_key: [u32; 8], // 256bit (at least bigger then u8::MAX)
}

impl Keyboard {
	pub const fn new() -> Self {
		Keyboard {
			driver: None,
			pressed_key: [0; 8],
		} // false, false, false ...
	}

	pub fn attach(&mut self, driver: Arc<dyn KbdDriver>) -> Result<(), Errno> {
		if let Some(_) = self.driver {
			return Err(Errno::EBUSY);
		}

		self.driver = Some(driver);

		Ok(())
	}

	pub fn detach(&mut self) {
		self.driver = None;
	}

	pub fn reset_cpu(&self) {
		if let Some(ref driver) = self.driver {
			driver.reset_cpu();
		}
	}

	/// 키보드에서 키 하나를 입력받고, 상태를 저장한 후, 받은 키를 반환한다.
	///
	/// # Returns
	///  - `None` -> 현재 키보드 버퍼에서 읽을 키가 존재하지 않음.
	///  - `Some(x)` -> 읽은 키에 대한 정보
	pub fn get_keyboard_event(&mut self) -> Option<KeyEvent> {
		let driver = self.driver.as_ref()?;

		let event = driver.get_key_event()?;

		self.change_state(event);
		Some(event)
	}

	/// wait until key is pressed, then return received event.
	pub fn wait_keyboard_event(&mut self) -> KeyEvent {
		loop {
			if let Some(ev) = self.get_keyboard_event() {
				return ev;
			}
		}
	}

	/// 현재 키  `code` 가 눌린 상태인지 검사
	pub fn pressed(&self, code: Code) -> bool {
		let (arr, bit) = Self::bit_index(code as u8);

		(self.pressed_key[arr] & (1 << bit)) != 0
	}

	pub fn shift_pressed(&self) -> bool {
		self.pressed(Code::LShift) || self.pressed(Code::RShift)
	}

	pub fn gui_pressed(&self) -> bool {
		self.pressed(Code::LGui) || self.pressed(Code::RGui)
	}

	pub fn control_pressed(&self) -> bool {
		self.pressed(Code::LControl) || self.pressed(Code::RControl)
	}

	pub fn alt_pressed(&self) -> bool {
		self.pressed(Code::LAlt) || self.pressed(Code::RAlt)
	}

	fn bit_index(idx: u8) -> (usize, usize) {
		(idx as usize / 32, idx as usize % 32)
	}

	fn clear_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.pressed_key[arr] &= !(1 << bit);
	}

	fn set_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.pressed_key[arr] |= 1 << bit;
	}

	fn toggle_state_at(&mut self, idx: u8) {
		let (arr, bit) = Self::bit_index(idx);

		self.pressed_key[arr] ^= 1 << bit;
	}

	fn change_state(&mut self, event: KeyEvent) {
		let code = event.key;

		// pause doesn't have press / release state.
		if let Code::Pause = code {
			return;
		}

		if let KeyKind::Toggle(..) = event.identify() {
			if event.pressed() {
				self.toggle_state_at(code as u8);
			}
		} else {
			if event.pressed() {
				self.set_state_at(code as u8);
			} else {
				self.clear_state_at(code as u8);
			}
		}
	}
}

pub fn change_state(event: KeyEvent) {
	unsafe { KEYBOARD.change_state(event) }
}
