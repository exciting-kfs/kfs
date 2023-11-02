use core::mem::{self};

use alloc::sync::Arc;

use crate::config::{USTACK_BASE, USTACK_PAGES};
use crate::elf::Elf;
use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, AccessFlag, IOFlag, Permission, RealEntry};
use crate::interrupt::InterruptFrame;
use crate::mm::user::memory::Memory;
use crate::mm::user::verify::verify_path;
use crate::process::task::{Task, CURRENT};
use crate::ptr::VirtPageBox;
use crate::syscall::errno::Errno;

const PATH_MAX: usize = 128;

fn read_user_binary(path: Path, task: &Arc<Task>) -> Result<VirtPageBox, Errno> {
	let entry = lookup_entry_follow(&path, task).and_then(|x| x.downcast_file())?;

	entry.access(Permission::ANY_EXECUTE, task)?;

	let stat = entry.stat()?;
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

	let raw_bin = read_user_binary(path, current)?;
	let elf = Elf::new(raw_bin.as_slice()).map_err(|_| Errno::ENOEXEC)?;
	let entry_point = elf.get_entry_point();

	let mut new_memory = Memory::from_elf(USTACK_BASE, USTACK_PAGES, elf)?;

	let argv = new_memory.push_string_array(argv, current)?;
	let envp = new_memory.push_string_array(envp, current)?;

	new_memory.pick_up();

	let mut memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	mem::drop(mem::replace(&mut *memory, new_memory));

	unsafe {
		frame.copy_from_nonoverlapping(&InterruptFrame::new_user(entry_point, USTACK_BASE), 1);

		let argc = argv.len() - 1;
		for x in Some(argc)
			.into_iter()
			.chain(argv.into_iter())
			.chain(envp.into_iter())
			.chain(Some(0))
			.rev()
		{
			(*frame).esp -= 4;
			((*frame).esp as *mut usize).write(x);
		}
	};

	Ok(0)
}
