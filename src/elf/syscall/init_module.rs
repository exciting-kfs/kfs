use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::elf::kobject::{KernelModule, KernelObject};
use crate::elf::{Elf, ElfError};
use crate::fs::path::Path;
use crate::fs::vfs::{lookup_entry_follow, AccessFlag, IOFlag, RealEntry};
use crate::mm::user::verify::verify_path;
use crate::process::task::CURRENT;
use crate::ptr::VirtPageBox;
use crate::sync::Locked;
use crate::syscall::errno::Errno;

static LOADED_MODULES: Locked<BTreeMap<Vec<u8>, Arc<KernelModule>>> = Locked::new(BTreeMap::new());

pub fn sys_init_module(path_ptr: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path_ptr, current)?;
	let mut path = Path::new(path);

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

	let module_name = path.pop_component().unwrap();

	do_sys_init_module(&elf_buf, module_name).map_err(|x| x.into())
}

fn do_sys_init_module(elf_buf: &[u8], name: Vec<u8>) -> Result<usize, ElfError> {
	let elf = Elf::new(elf_buf)?;

	let kobject = KernelObject::new(&elf)?;
	let module = Arc::new(kobject.load()?);

	let ep = module.get_entry_point();

	unsafe { (*(&ep as *const _ as usize as *const extern "C" fn()))() }

	LOADED_MODULES.lock().insert(name, module);

	Ok(0)
}
