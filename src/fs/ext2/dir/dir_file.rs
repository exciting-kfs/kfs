use core::{
	mem::{size_of, transmute},
	ptr::copy_nonoverlapping,
};

use crate::{
	fs::vfs::{self, IOFlag, KfsDirent},
	handle_r_iter_error,
	mm::util::next_align,
	syscall::errno::Errno,
	trace_feature,
};

use super::{dir_inode::DirInode, Dirent, Iter};

pub struct DirFile {
	inode: DirInode,
}

impl DirFile {
	pub fn new(inode: DirInode) -> Self {
		Self { inode }
	}
}

impl vfs::DirHandle for DirFile {
	fn getdents(&self, buf: &mut [u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		let non_block = flags.contains(IOFlag::O_NONBLOCK);

		let mut sum = 0;
		let mut iter = Iter::new(&self.inode, 0);

		loop {
			let chunk = iter.next();

			if let Ok(chunk) = chunk {
				sum += write_to_buf(buf, chunk, sum)?;
			} else {
				handle_r_iter_error!(chunk.unwrap_err(), non_block);
			}
		}

		Ok(sum)
	}

	fn close(&self) -> Result<(), Errno> {
		self.inode.inner().sync()
	}
}

fn write_to_buf(buf: &mut [u8], chunk: Dirent, sum: usize) -> Result<usize, Errno> {
	let name = chunk.get_name();
	let record = chunk.get_record();
	let size = next_align(KfsDirent::total_len(&name), 4);

	trace_feature!("ext2-getdents"
		"name: {:?}, record: {:?}",
		String::from_utf8(name.iter().map(|e| *e).collect::<Vec<u8>>()),
		*record
	);

	if sum + size > buf.len() {
		return Err(Errno::EINVAL);
	}

	let dirent: [u8; size_of::<KfsDirent>()] = unsafe {
		transmute(KfsDirent {
			ino: record.ino,
			private: 0,
			size: size as u16,
			file_type: record.file_type,
			name: (),
		})
	};

	unsafe {
		let header_len = KfsDirent::header_len();
		let ptr = buf.as_mut_ptr().offset(sum as isize);
		copy_nonoverlapping(dirent.as_ptr(), ptr, header_len);

		let ptr = ptr.offset(header_len as isize);
		copy_nonoverlapping(name.as_ptr(), ptr, name.len());

		let ptr = ptr.offset(name.len() as isize);
		let remains = size - header_len - name.len();
		(0..remains).for_each(|i| ptr.offset(i as isize).write(0));
	}
	Ok(size)
}
