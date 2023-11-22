use core::ptr::addr_of_mut;

use crate::{
	fs::vfs::{self, IOFlag, KfsDirent},
	handle_r_iter_error,
	mm::util::next_align,
	sync::LocalLocked,
	syscall::errno::Errno,
	trace_feature,
};

use super::{dir_inode::DirInode, Dirent, Iter};

pub struct DirFile {
	inode: DirInode,
	iter: LocalLocked<Iter>,
}

impl DirFile {
	pub fn new(inode: &DirInode) -> Self {
		let iter = LocalLocked::new(Iter::new(inode, 0));
		Self {
			inode: inode.clone(),
			iter,
		}
	}
}

impl vfs::DirHandle for DirFile {
	fn getdents(&self, buf: &mut [u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		let non_block = flags.contains(IOFlag::O_NONBLOCK);

		trace_feature!(
			"time-ext2-getdents",
			"start: {}",
			crate::driver::hpet::get_timestamp_mili() % 1000
		);

		let mut sum = 0;
		let mut iter = self.iter.lock();

		trace_feature!(
			"time-ext2-getdents",
			"after lock: {}",
			get_timestamp_mili() % 1000
		);

		let mut chunk = iter.next();

		if let Ok(chunk) = chunk.as_ref() {
			let name = chunk.get_name();
			let size = KfsDirent::total_len(&name);

			if size > buf.len() {
				return Err(Errno::EINVAL);
			}
		}

		loop {
			if let Ok(chunk) = chunk {
				let res = write_to_buf(buf, chunk, sum);
				if res == 0 {
					iter.rewind();
					break;
				} else {
					sum += res;
				}
			} else {
				handle_r_iter_error!(chunk.unwrap_err(), non_block);
			}

			trace_feature!(
				"time-ext2-getdents",
				"one entry end: {}",
				crate::driver::hpet::get_timestamp_mili() % 1000
			);

			chunk = iter.next();
		}

		Ok(sum)
	}

	fn close(&self) -> Result<(), Errno> {
		self.inode.inner().sync()
	}
}

fn write_to_buf(buf: &mut [u8], chunk: Dirent, sum: usize) -> usize {
	let name = chunk.get_name();
	let record = chunk.get_record();
	let size = next_align(KfsDirent::total_len(&name), 8);

	trace_feature!(
		"ext2-getdents",
		"name: {:?}, record: {:?}",
		alloc::string::String::from_utf8((&*name).to_vec()),
		*record,
	);

	if sum + size > buf.len() || name.len() == 0 {
		return 0;
	}

	unsafe {
		let ptr = buf.as_mut_ptr().add(sum).cast::<KfsDirent>();

		ptr.write(KfsDirent {
			ino: record.ino as u64,
			private: 0,
			size: size as u16,
			file_type: record.file_type,
			name: (),
		});

		let ptr = addr_of_mut!((*ptr).name).cast::<u8>();
		ptr.copy_from_nonoverlapping(name.as_ptr(), name.len());

		let ptr = ptr.add(name.len());
		ptr.write(0);
	}
	size
}
