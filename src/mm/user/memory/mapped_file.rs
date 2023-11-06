use core::slice::from_raw_parts;

use crate::{
	fs::vfs::{VfsHandle, Whence},
	syscall::errno::Errno,
};

#[derive(Clone)]
pub struct MappedFile {
	file: VfsHandle,
	offset: isize,
	len: usize,
}

impl MappedFile {
	pub fn new(file: VfsHandle, offset: isize, len: usize) -> Self {
		Self { file, offset, len }
	}

	pub fn sync_with_buf(&self, buf: *const u8) -> Result<(), Errno> {
		self.file.lseek(self.offset, Whence::Begin)?;

		let buf = unsafe { from_raw_parts(buf, self.len) };

		let mut cursor = 0;
		while cursor < self.len {
			let buf = &buf[cursor..];
			let x = self.file.write(buf)?;
			cursor += x;
		}
		Ok(())
	}
}
