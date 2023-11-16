use crate::{mm::user::verify::verify_array_mut, pr_debug, process::task::CURRENT};

use super::errno::Errno;

#[derive(Debug)]
#[repr(C)]
struct PollFd {
	fd: i32,
	events: i16,
	revents: i16,
}

pub fn sys_poll(fds: usize, nfds: usize, _timeout: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let fds = verify_array_mut::<PollFd>(fds, nfds, current)?;

	pr_debug!("POLL: {:?}", fds);

	Ok(nfds)
}
