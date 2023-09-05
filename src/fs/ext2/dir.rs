use crate::{fs::vfs, syscall::errno::Errno};

struct Dir {}

#[allow(unused)]
impl vfs::DirHandle for Dir {
	fn getdents(&self, buf: &mut [u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		todo!()
	}
}
