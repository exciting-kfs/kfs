use core::marker::PhantomData;
use core::mem::{self, size_of};

use crate::collection::WrapQueue;
use crate::file::{File, FileOps, OpenFlag};
use crate::interrupt::syscall::errno::Errno;
use crate::mm::user::vma::AreaFlag;
use crate::process::context::yield_now;
use crate::process::task::CURRENT;
use crate::signal::send_signal_to;
use crate::signal::sig_code::SigCode;
use crate::signal::sig_info::SigInfo;
use crate::signal::sig_num::SigNum;
use crate::sync::locked::{Locked, LockedGuard};

use alloc::sync::Arc;

trait PipeEnd {}

pub struct WriteEnd;
pub struct ReadEnd;

impl PipeEnd for WriteEnd {}
impl PipeEnd for ReadEnd {}

struct PipeBuffer {
	pub data: WrapQueue<u8, 4096>,
	pub widowed: bool,
}

impl PipeBuffer {
	fn new() -> Self {
		Self {
			data: WrapQueue::new(),
			widowed: false,
		}
	}
}

struct Pipe<T: PipeEnd> {
	buffer: Arc<Locked<PipeBuffer>>,
	kind: PhantomData<T>,
}

impl<T: PipeEnd> Pipe<T> {
	fn new(buffer: Arc<Locked<PipeBuffer>>) -> Self {
		Self {
			buffer,
			kind: PhantomData,
		}
	}

	fn lock_buffer(&self) -> LockedGuard<'_, PipeBuffer> {
		loop {
			match self.buffer.try_lock() {
				Ok(x) => break x,
				_ => yield_now(),
			};
		}
	}
}

impl FileOps for Pipe<ReadEnd> {
	fn read(&self, file: &Arc<File>, buf: &mut [u8]) -> Result<usize, Errno> {
		let total_len = buf.len();

		let mut out_buf = buf;
		let mut read = 0;
		while out_buf.len() != 0 {
			let mut pipe_buf = self.lock_buffer();

			read += pipe_buf.data.read(out_buf);

			if pipe_buf.widowed {
				return Ok(read);
			}

			if file.open_flag.contains(OpenFlag::O_NONBLOCK) {
				match read {
					0 => return Err(Errno::EAGAIN),
					x => return Ok(x),
				}
			}

			let (_, remain) = out_buf.split_at_mut(read);
			out_buf = remain;

			mem::drop(pipe_buf);
			yield_now();
		}

		Ok(total_len)
	}

	fn write(&self, _file: &Arc<File>, _buf: &[u8]) -> Result<usize, Errno> {
		return Err(Errno::EBADF);
	}
}

impl FileOps for Pipe<WriteEnd> {
	fn read(&self, _file: &Arc<File>, _buf: &mut [u8]) -> Result<usize, Errno> {
		return Err(Errno::EBADF);
	}

	fn write(&self, file: &Arc<File>, buf: &[u8]) -> Result<usize, Errno> {
		let total_len = buf.len();

		let mut in_buf = buf;
		while in_buf.len() != 0 {
			let mut pipe_buf = self.lock_buffer();

			if pipe_buf.widowed {
				let siginfo = SigInfo {
					num: SigNum::PIPE,
					pid: 0,
					uid: 0,
					code: SigCode::SI_KERNEL,
				};

				let current = unsafe { CURRENT.get_mut() };

				let _ = send_signal_to(current, &siginfo);

				return Err(Errno::EPIPE);
			}

			let write = pipe_buf.data.write(in_buf);

			if file.open_flag.contains(OpenFlag::O_NONBLOCK) {
				match write {
					0 => return Err(Errno::EAGAIN),
					x => return Ok(x),
				}
			}

			let (_, remain) = in_buf.split_at(write);
			in_buf = remain;

			mem::drop(pipe_buf);
			yield_now();
		}

		Ok(total_len)
	}
}

impl<T: PipeEnd> Drop for Pipe<T> {
	fn drop(&mut self) {
		let mut buffer = self.buffer.lock();

		buffer.widowed = true;
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
			let _ = fd_table.close(x);

			Err(Errno::EMFILE)
		}
		(None, None) => Err(Errno::EMFILE),
	}
}
