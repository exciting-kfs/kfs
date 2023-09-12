use crate::{fs::vfs, syscall::errno::Errno};

pub struct File {}

#[allow(unused)]
impl vfs::FileHandle for File {
	fn lseek(&self, offset: isize, whence: vfs::Whence) -> Result<usize, Errno> {
		todo!()
	}
	fn read(&self, buf: &mut [u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		todo!()
	}
	fn write(&self, buf: &[u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		todo!()
	}
}
