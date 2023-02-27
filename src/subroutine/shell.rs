use crate::io::character::{Read, Write, RW};

pub static mut SHELL: [Shell; 3] = [Shell, Shell, Shell];
pub struct Shell;

impl Read<u8> for Shell {
	fn read_one(&mut self) -> Option<u8> {
		None
	}
}

impl Write<u8> for Shell {
	fn write_one(&mut self, data: u8) {}
}

impl RW<u8, u8> for Shell {}
