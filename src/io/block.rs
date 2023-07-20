use super::character::{Read as ChRead, Write as ChWrite};

pub trait Read: ChRead<u8> {
	fn read(&mut self, buf: &mut [u8]) -> usize {
		for (i, b) in buf.iter_mut().enumerate() {
			match self.read_one() {
				Some(c) => *b = c,
				None => return i,
			}
		}
		buf.len()
	}
}

pub trait Write: ChWrite<u8> {
	fn write(&mut self, buf: &[u8]) -> usize {
		for (i, b) in buf.iter().enumerate() {
			match self.write_one(*b) {
				Ok(_) => {}
				Err(_) => return i,
			}
		}
		buf.len()
	}
}

pub trait RW: Read + Write {}
