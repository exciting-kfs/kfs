use core::marker::PhantomData;
use core::mem::size_of;

use crate::collection::WrapQueue;
use crate::file::{File, FileOps, OpenFlag};
use crate::interrupt::syscall::errno::Errno;
use crate::mm::user::vma::AreaFlag;
use crate::process::context::yield_now;
use crate::process::task::CURRENT;
use crate::sync::locked::Locked;

use alloc::sync::Arc;

pub struct WriteEnd;
pub struct ReadEnd;

type PipeBuffer = WrapQueue<u8, 4096>;

pub struct Pipe<T> {
	buffer: Arc<Locked<PipeBuffer>>,
	kind: PhantomData<T>,
}

impl<T> Pipe<T> {
	fn new(buffer: Arc<Locked<PipeBuffer>>) -> Self {
		Self {
			buffer,
			kind: PhantomData,
		}
	}
}

impl FileOps for Pipe<ReadEnd> {
	fn read(&self, _file: &Arc<File>, buf: &mut [u8]) -> Result<usize, Errno> {
		let mut pipe_buf = loop {
			match self.buffer.try_lock() {
				Ok(x) => break x,
				_ => yield_now(),
			}
		};

		Ok(pipe_buf.read(buf))
	}

	fn write(&self, _file: &Arc<File>, _buf: &[u8]) -> Result<usize, Errno> {
		return Err(Errno::EBADF);
	}
}

impl FileOps for Pipe<WriteEnd> {
	fn read(&self, _file: &Arc<File>, _buf: &mut [u8]) -> Result<usize, Errno> {
		return Err(Errno::EBADF);
	}

	fn write(&self, _file: &Arc<File>, buf: &[u8]) -> Result<usize, Errno> {
		let mut pipe_buf = loop {
			match self.buffer.try_lock() {
				Ok(x) => break x,
				_ => yield_now(),
			}
		};

		Ok(pipe_buf.write(buf))
	}
}


pub fn get_pipe() -> (Arc<File>, Arc<File>) {
	let buffer = Arc::new(Locked::new(PipeBuffer::new()));

	(
		Arc::new(File::new(
			Arc::new(Pipe::<ReadEnd>::new(buffer.clone())),
			OpenFlag::empty(),
		)),
		Arc::new(File::new(
			Arc::new(Pipe::<WriteEnd>::new(buffer.clone())),
			OpenFlag::empty(),
		)),
	)
}

pub fn sys_pipe(pipe_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let user_ext = current.get_user_ext().expect("must be user task");

	if !user_ext
		.lock_memory()
		.query_flags_range(pipe_ptr, 2 * size_of::<i32>(), AreaFlag::Writable)
	{
		return Err(Errno::EFAULT);
	}

	let pipe = get_pipe();

	let mut fd_table = user_ext.lock_fd_table();

	let read_end = fd_table.alloc_fd(pipe.0.clone());
	let write_end = fd_table.alloc_fd(pipe.1.clone());

	match (read_end, write_end) {
		(Some(x), Some(y)) => {
			let pipe = unsafe { &mut *(pipe_ptr as *mut [i32; 2]) };

			pipe[0] = x.index() as i32;
			pipe[1] = y.index() as i32;

			Ok(0)
		}
		(Some(x), None) | (None, Some(x)) => {
			fd_table.close(x);

			Err(Errno::EMFILE)
		}
		(None, None) => Err(Errno::EMFILE),
	}
}
