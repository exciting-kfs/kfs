use core::{
	mem::{size_of, transmute},
	ptr::copy_nonoverlapping,
};

use alloc::{string::String, vec::Vec};

use crate::{
	fs::{
		ext2::inode::IterError,
		vfs::{self, IOFlag, KfsDirent},
	},
	handle_iter_error,
	mm::util::next_align,
	pr_debug,
	process::signal::poll_signal_queue,
	scheduler::sleep::Sleep,
	syscall::errno::Errno,
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
				handle_iter_error!(chunk.unwrap_err(), non_block);
			}
		}

		pr_debug!("getdent: end");

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

	let s = name.iter().map(|e| *e).collect::<Vec<u8>>();
	let str = String::from_utf8(s);
	pr_debug!("name: {:?}, record: {:?}", str, *record);

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
