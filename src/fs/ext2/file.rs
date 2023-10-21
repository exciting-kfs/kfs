use core::ptr::copy_nonoverlapping;

use alloc::{boxed::Box, sync::Arc, vec::Vec};

use crate::{
	fs::{
		ext2::inode::info::InodeInfoMut,
		vfs::{self, IOFlag, Permission},
	},
	handle_r_iter_error, handle_w_iter_error,
	mm::util::next_align,
	scheduler::sleep::Sleep,
	sync::{LockRW, Locked},
	syscall::errno::Errno,
	trace_feature,
};

use super::{
	inode::{self, inum::Inum, Inode},
	sb::SuperBlock,
};

pub struct File {
	cursor: Locked<usize>,
	inode: FileInode,
}

impl File {
	pub fn new(inode: FileInode) -> Self {
		Self {
			cursor: Locked::new(0),
			inode,
		}
	}
}

#[allow(unused)]
impl vfs::FileHandle for File {
	fn lseek(&self, offset: isize, whence: vfs::Whence) -> Result<usize, Errno> {
		let cursor = *self.cursor.lock();
		let size = self.inode.inner().read_lock().size();

		let new_cursor = {
			let new_cursor = match whence {
				vfs::Whence::Begin => offset,
				vfs::Whence::End => size as isize + offset,
				vfs::Whence::Current => cursor as isize + offset,
			};

			if new_cursor < 0 || (size as isize) < new_cursor {
				return Err(Errno::EINVAL);
			}

			new_cursor
		};

		*self.cursor.lock() = new_cursor as usize;

		Ok(new_cursor as usize)
	}

	fn read(&self, buf: &mut [u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		let non_block = flags.contains(IOFlag::O_NONBLOCK);
		let length = buf.len();

		let mut sum = 0;
		let mut iter = inode::Iter::new(self.inode.inner().clone(), *self.cursor.lock());

		while sum < length {
			let req_len = length - sum;
			let chunk = iter.next(req_len);

			if let Ok(chunk) = chunk {
				sum += write_to_user(buf, &chunk.slice(), sum);
			} else {
				handle_r_iter_error!(chunk.unwrap_err(), non_block)
			}
		}

		*self.cursor.lock() += sum;
		Ok(sum)
	}

	fn write(&self, buf: &[u8], flags: vfs::IOFlag) -> Result<usize, Errno> {
		let non_block = flags.contains(IOFlag::O_NONBLOCK);
		let inode = self.inode.inner();

		if flags.contains(vfs::IOFlag::O_APPEND) {
			*self.cursor.lock() = inode.read_lock().size();
		}

		let length = buf.len();
		let mut sum = 0;
		let mut iter = inode::Iter::new(inode.clone(), *self.cursor.lock());

		while sum < length {
			let req_len = length - sum;
			let cursor = iter.cursor();
			let chunk = iter.next_mut(req_len);

			if let Ok(chunk) = chunk {
				sum += write_to_file(buf, &mut chunk.slice_mut(), sum);

				if flags.contains(vfs::IOFlag::O_SYNC) {
					inode.sync()?;
				}
			} else {
				handle_w_iter_error!(chunk.unwrap_err(), non_block)
			}
		}

		*self.cursor.lock() += sum;
		Ok(sum)
	}

	fn close(&self) -> Result<(), Errno> {
		self.inode.inner().sync()
	}
}

fn write_to_user(u_buf: &mut [u8], k_buf: &[u8], read_sum: usize) -> usize {
	let read_size = k_buf.len();
	unsafe {
		copy_nonoverlapping(
			k_buf.as_ptr(),
			u_buf.as_mut_ptr().offset(read_sum as isize),
			read_size,
		);
	}

	read_size
}

fn write_to_file(u_buf: &[u8], k_buf: &mut [u8], write_sum: usize) -> usize {
	let write_size = k_buf.len();
	unsafe {
		copy_nonoverlapping(
			u_buf.as_ptr().offset(write_sum as isize),
			k_buf.as_mut_ptr(),
			write_size,
		);
	}

	write_size
}

#[derive(Clone)]
pub struct FileInode(Arc<LockRW<Inode>>);

impl FileInode {
	pub fn from_inode(inode: Arc<LockRW<Inode>>) -> Self {
		Self(inode)
	}

	pub fn new_shared(sb: &Arc<SuperBlock>, inum: Inum, perm: Permission) -> Arc<Self> {
		let inode = Inode::new_file(inum, sb, perm);
		let inode = Arc::new(LockRW::new(inode));
		sb.inode_cache.lock().insert(inum, inode.clone());
		inode.read_lock().dirty();
		Arc::new(Self(inode))
	}

	pub fn inner(&self) -> &Arc<LockRW<Inode>> {
		&self.0
	}

	fn expand(&self, old_idx: usize, new_idx: usize) -> Result<(), Errno> {
		let mut iter = inode::Iter::new(self.inner().clone(), old_idx);
		let mut total = new_idx - old_idx;
		while total > 0 {
			let chunk = iter.next_mut_block(total)?;
			let mut slice = chunk.slice_mut();

			slice.fill(0);

			total -= slice.len();
		}

		trace_feature!("ext2-truncate", "file: expand: {}", new_idx);

		Ok(())
	}

	fn shrink(&self, new_idx: usize) -> Result<(), Errno> {
		let sb = self.inner().super_block();

		let block_size = sb.block_size();
		let new_len = next_align(new_idx, block_size) / block_size;

		let mut data = self.inner().data_write();
		let to_dealloc = data.chunks_range(new_len..);

		let mut staged = Vec::new();

		for bid in to_dealloc.iter().map(|b| *b.lock().block_id()) {
			staged.push(sb.dealloc_block_staged(bid)?);
		}

		staged.into_iter().for_each(|s| s.commit(()));
		data.truncate(new_len);

		InodeInfoMut::from_data(data).set_size(new_idx);

		trace_feature!("ext2-truncate", "file: shrink: {}", new_idx);

		Ok(())
	}
}

impl vfs::RealInode for FileInode {
	fn stat(&self) -> Result<vfs::RawStat, Errno> {
		Ok(self.inner().info().stat())
	}

	fn chmod(&self, perm: vfs::Permission) -> Result<(), Errno> {
		self.inner().info_mut().chmod(perm)
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		self.inner().info_mut().chown(owner, group)
	}
}

#[allow(unused)]
impl vfs::FileInode for FileInode {
	fn open(&self) -> Result<Box<dyn vfs::FileHandle>, Errno> {
		self.inner().load_bid()?;
		Ok(Box::new(File::new(self.clone())))
	}

	fn truncate(&self, length: isize) -> Result<(), Errno> {
		if length < 0 {
			return Err(Errno::EINVAL);
		}

		let (old_idx, new_idx) = {
			let r_inode = self.inner().read_lock();
			(r_inode.size(), length as usize)
		};

		let sb = self.inner().super_block();
		if old_idx < new_idx {
			self.expand(old_idx, new_idx)?;
			vfs::SuperBlock::sync(sb.as_ref())?;
		} else if old_idx > new_idx {
			self.shrink(new_idx)?;
			vfs::SuperBlock::sync(sb.as_ref())?;
		}
		Ok(())
	}
}
