use core::ffi::CStr;
use core::mem::{self};

use alloc::borrow::ToOwned;
use alloc::sync::Arc;

use crate::elf::Elf;
use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, AccessFlag, Entry, IOFlag, Permission};
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::mm::user::string_vec::StringVec;
use crate::mm::user::verify::verify_path;
use crate::process::task::{Task, CURRENT};
use crate::ptr::VirtPageBox;
use crate::syscall::errno::Errno;
use crate::syscall::SyscallSnapshot;
use crate::trace_feature;

const PATH_MAX: usize = 128;

pub fn read_user_binary(path: Path, task: &Arc<Task>) -> Result<VirtPageBox, Errno> {
	let entry = lookup_entry_follow(&path, task).and_then(|x| x.downcast_file())?;

	entry.access(Permission::ANY_EXECUTE, task)?;

	let stat = entry.statx()?;

	let mut buffer = VirtPageBox::new(stat.size as usize).map_err(|_| Errno::ENOMEM)?;

	let handle = entry.open(IOFlag::empty(), AccessFlag::O_RDONLY)?;
	handle.read(&mut buffer[..stat.size as usize])?;

	Ok(buffer)
}

/// execute new user binary
/// do not call from kernel thread!!
pub fn sys_execve(
	frame: *mut InterruptFrame,
	path_ptr: usize,
	argv: usize,
	envp: usize,
) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path_ptr, current)?;
	let path = Path::new(path);

	trace_feature!(
		"syscall",
		"{:?}: {} #P: {}",
		unsafe { CURRENT.get_ref().get_pid() },
		SyscallSnapshot::new(unsafe { &*frame }),
		path
	);

	let raw_bin = read_user_binary(path, current)?;
	let elf = Elf::new(raw_bin.as_slice()).map_err(|_| Errno::ENOEXEC)?;

	let argv = StringVec::new(argv, current)?;
	let envp = StringVec::new(envp, current)?;

	let new_cmd = CStr::from_bytes_until_nul(&argv.data)
		.map(|s| s.to_owned().into_bytes())
		.unwrap_or_default();

	let new_memory = Memory::from_elf(elf, argv, envp)?;

	let signal = &current.user_ext_ok_or(Errno::EPERM)?.signal;
	signal.do_for_exec();

	new_memory.pick_up();

	let mut memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	unsafe {
		frame.copy_from_nonoverlapping(
			&InterruptFrame::new_user(new_memory.entry_point, new_memory.get_stack_pointer()),
			1,
		);
	};

	mem::drop(mem::replace(&mut *memory, new_memory));

	*current.lock_cmd() = new_cmd;

	Ok(0)
}
