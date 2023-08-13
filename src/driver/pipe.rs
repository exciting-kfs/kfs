use core::marker::PhantomData;
use core::mem::{self, size_of};

use crate::collection::WrapQueue;
use crate::file::{File, FileOps, OpenFlag};
use crate::mm::user::vma::AreaFlag;
use crate::process::context::yield_now;
use crate::process::signal::send_signal_to;
use crate::process::signal::sig_code::SigCode;
use crate::process::signal::sig_info::SigInfo;
use crate::process::signal::sig_num::SigNum;
use crate::process::task::CURRENT;
use crate::sync::locked::{Locked, LockedGuard};
use crate::syscall::errno::Errno;

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
		let mut out_buf = buf;
		let mut total_read = 0;
		while out_buf.len() != 0 {
			let mut pipe_buf = self.lock_buffer();

			let curr_read = pipe_buf.data.read(out_buf);
			total_read += curr_read;

			if pipe_buf.widowed {
				return Ok(total_read);
			}

			if file.open_flag.contains(OpenFlag::O_NONBLOCK) {
				match curr_read {
					0 => return Err(Errno::EAGAIN),
					x => return Ok(x),
				}
			}

			let (_, remain) = out_buf.split_at_mut(curr_read);
			out_buf = remain;

			mem::drop(pipe_buf);
			yield_now();
		}

		Ok(total_read)
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
		let mut in_buf = buf;
		let mut total_write = 0;
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

			let curr_write = pipe_buf.data.write(in_buf);
			total_write += curr_write;

			if file.open_flag.contains(OpenFlag::O_NONBLOCK) {
				match curr_write {
					0 => return Err(Errno::EAGAIN),
					x => return Ok(x),
				}
			}

			let (_, remain) = in_buf.split_at(curr_write);
			in_buf = remain;

			mem::drop(pipe_buf);
			yield_now();
		}

		Ok(total_write)
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
