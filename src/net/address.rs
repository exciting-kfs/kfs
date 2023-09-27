use core::mem::size_of;

use alloc::sync::Arc;

use crate::mm::user::verify::{verify_buffer, verify_buffer_mut};
use crate::{process::task::Task, syscall::errno::Errno};

#[repr(C)]
pub struct SocketAddressHeader {
	pub kind: u16,
	pub data: [u8; 0],
}

pub trait AccessControl {}

pub struct WriteOnly;
impl AccessControl for WriteOnly {}

pub struct ReadOnly;
impl AccessControl for ReadOnly {}

pub struct UnknownSocketAddress<T: AccessControl> {
	pub len: usize,
	pub ptr: *const SocketAddressHeader,
	_access: T,
}

pub trait SocketAddress {
	fn from_unknown(unknown: &UnknownSocketAddress<ReadOnly>) -> Result<Self, Errno>
	where
		Self: Sized;

	fn copy_to_unknown(&self, unknown: &mut UnknownSocketAddress<WriteOnly>) -> Result<(), Errno>;
}

impl UnknownSocketAddress<WriteOnly> {
	pub fn new_write_only(
		addr: usize,
		len: usize,
		task: &Arc<Task>,
	) -> Result<Option<Self>, Errno> {
		if addr == 0 {
			return Ok(None);
		}

		if len < size_of::<SocketAddressHeader>() {
			return Err(Errno::EINVAL);
		}

		verify_buffer_mut(addr, len, task)?;

		Ok(Some(Self {
			len,
			ptr: addr as *const SocketAddressHeader,
			_access: WriteOnly,
		}))
	}

	pub fn copy_from<T: SocketAddress>(&mut self, solid: &Option<T>) -> Result<(), Errno> {
		if let Some(s) = solid.as_ref() {
			return s.copy_to_unknown(self);
		}

		self.ptr = 0 as *const SocketAddressHeader;
		self.len = 0;

		Ok(())
	}
}

impl UnknownSocketAddress<ReadOnly> {
	pub fn new_read_only(addr: usize, len: usize, task: &Arc<Task>) -> Result<Option<Self>, Errno> {
		if addr == 0 {
			return Ok(None);
		}

		if len < size_of::<SocketAddressHeader>() {
			return Err(Errno::EINVAL);
		}

		verify_buffer(addr, len, task)?;

		Ok(Some(Self {
			len,
			ptr: addr as *const SocketAddressHeader,
			_access: ReadOnly,
		}))
	}

	pub fn get_raw_domain(&self) -> u16 {
		unsafe { (*self.ptr).kind }
	}

	pub fn copy_to_solid<T: SocketAddress>(&self) -> Result<T, Errno> {
		T::from_unknown(self)
	}
}
