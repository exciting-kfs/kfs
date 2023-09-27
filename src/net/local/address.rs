use core::{mem::size_of, ptr::addr_of, slice::from_raw_parts};

use alloc::rc::Rc;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::fs::path::{Base, Path};
use crate::fs::vfs::{
	lookup_entry_follow, Permission, SocketInode, VfsEntry, VfsSocketEntry, VfsSocketHandle,
};
use crate::net::address::{
	ReadOnly, SocketAddress, SocketAddressHeader, UnknownSocketAddress, WriteOnly,
};
use crate::net::domain;
use crate::net::socket::SocketHandle;
use crate::process::task::Task;
use crate::sync::Locked;
use crate::{process::task::CURRENT, syscall::errno::Errno};

#[repr(C)]
#[derive(Clone)]
pub struct LocalSocketAddress {
	pub(super) path: Path,
}

impl LocalSocketAddress {
	pub fn to_buffer(&self) -> Vec<u8> {
		let mut buf: Vec<u8> = Vec::new();

		use Base::*;
		if let WorkingDir { to_parent } = self.path.base() {
			{
				if to_parent == 0 {
					buf.push(b'.');
				} else {
					for ch in core::iter::repeat(&b".."[..])
						.take(to_parent)
						.intersperse(&b"/"[..])
						.flatten()
					{
						buf.push(*ch);
					}
				}
			}
		}

		for comp in self.path.components() {
			buf.push(b'/');
			for ch in comp {
				buf.push(*ch);
			}
		}

		buf
	}

	pub fn lookup_socket(&self) -> Result<SocketHandle, Errno> {
		let current = unsafe { CURRENT.get_ref() };

		let entry = lookup_entry_follow(&self.path, current)?;
		let socket_entry = entry.downcast_socket()?;
		let socket_handle = socket_entry.get_socket()?;

		Ok(socket_handle.expose_socket())
	}
}

impl SocketAddress for LocalSocketAddress {
	fn from_unknown(unknown: &UnknownSocketAddress<ReadOnly>) -> Result<Self, Errno> {
		if unknown.get_raw_domain() as i32 != domain::code::LOCAL {
			return Err(Errno::EINVAL);
		}

		let buf = unsafe {
			from_raw_parts(
				addr_of!((*unknown.ptr).data).cast::<u8>(),
				unknown.len - size_of::<SocketAddressHeader>(),
			)
		};

		let len = buf.iter().position(|x| *x == 0).unwrap_or(buf.len());
		let (path, _) = buf.split_at(len);

		Ok(LocalSocketAddress {
			path: Path::new(path),
		})
	}

	fn copy_to_unknown(&self, unknown: &mut UnknownSocketAddress<WriteOnly>) -> Result<(), Errno> {
		let path_buf = self.to_buffer();

		let total_len = size_of::<SocketAddressHeader>() + path_buf.len();

		if total_len > unknown.len {
			return Err(Errno::EINVAL);
		}

		unknown.len = total_len;
		unsafe {
			unknown
				.ptr
				.cast::<u8>()
				.cast_mut()
				.copy_from_nonoverlapping(path_buf.as_ptr(), path_buf.len())
		};

		Ok(())
	}
}

fn local_socket_bind(
	addr: &UnknownSocketAddress<ReadOnly>,
	handle: &Arc<VfsSocketHandle>,
	task: &Arc<Task>,
) -> Result<LocalSocketAddress, Errno> {
	let addr: LocalSocketAddress = addr.copy_to_solid()?;

	let mut path = addr.path.clone();

	let socket_name = path.pop_component().ok_or(Errno::EISDIR)?;

	let base_entry = lookup_entry_follow(&path, task).and_then(|ent| ent.downcast_dir())?;

	let socket_inode = Arc::new(SocketInode::new(
		Permission::from_bits_truncate(0o755),
		task.get_uid(),
		task.get_gid(),
	));

	let socket = VfsEntry::new_socket(Arc::new(VfsSocketEntry::new(
		Rc::new(socket_name),
		socket_inode,
		Arc::downgrade(handle).clone(),
		Arc::downgrade(&base_entry),
	)));

	base_entry.insert_child_force(socket);

	Ok(addr)
}

pub(super) struct BindAddress {
	address: Locked<Option<LocalSocketAddress>>,
}

impl BindAddress {
	pub fn new() -> Self {
		Self {
			address: Locked::new(None),
		}
	}

	pub fn take(&self) -> Self {
		let address = self.address.lock().take();
		Self {
			address: Locked::new(address),
		}
	}

	pub fn bind(
		&self,
		addr: &UnknownSocketAddress<ReadOnly>,
		handle: &Arc<VfsSocketHandle>,
		task: &Arc<Task>,
	) -> Result<(), Errno> {
		let new_addr = local_socket_bind(addr, handle, task)?;

		let mut current_addr = self.address.lock();

		if current_addr.is_some() {
			return Err(Errno::EINVAL);
		}

		current_addr.replace(new_addr);

		Ok(())
	}

	pub fn clone_address(&self) -> Option<LocalSocketAddress> {
		self.address.lock().clone()
	}
}
