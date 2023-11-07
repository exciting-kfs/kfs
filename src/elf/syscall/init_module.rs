use alloc::sync::Arc;

use crate::elf::kobject::{load_kernel_module, KernelModule, KernelObject};
use crate::elf::{Elf, ElfError};
use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, AccessFlag, Entry, IOFlag};
use crate::mm::user::verify::verify_path;
use crate::process::task::CURRENT;
use crate::ptr::VirtPageBox;
use crate::syscall::errno::Errno;

pub fn sys_init_module(path_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	if !current.is_privileged() {
		return Err(Errno::EPERM);
	}

	let path = verify_path(path_ptr, current)?;
	let path = Path::new(path);

	let entry = lookup_entry_follow(&path, current)?;

	let stat = entry.stat()?;

	let file = entry.downcast_file()?;

	let fhandle = file.open(IOFlag::empty(), AccessFlag::O_RDONLY)?;

	let mut elf_buf = VirtPageBox::new(stat.size as usize).map_err(|_| Errno::ENOMEM)?;

	let mut offset = 0;
	while offset < stat.size as usize {
		let buf = &mut elf_buf.as_mut_slice()[offset..];
		let x = fhandle.read(buf)?;
		offset += x;
	}

	let module = parse_module_elf(&elf_buf).map_err(|x| <ElfError as Into<Errno>>::into(x))?;

	load_kernel_module(module).map(|_| 0)
}

fn parse_module_elf(elf_buf: &[u8]) -> Result<Arc<KernelModule>, ElfError> {
	let elf = Elf::new(elf_buf)?;

	let kobject = KernelObject::new(&elf)?;
	let module = kobject.load()?;

	Ok(module)
}
