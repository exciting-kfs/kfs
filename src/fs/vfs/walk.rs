use alloc::sync::Arc;

use crate::fs::path::{Base, Path};
use crate::fs::vfs::ROOT_DIR_ENTRY;
use crate::process::task::Task;
use crate::syscall::errno::Errno;

use super::{VfsDirEntry, VfsEntry, VfsFileEntry};

fn lookup_base_entry(base: Base, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
	let depth = match base {
		Base::RootDir => return ROOT_DIR_ENTRY.lock().clone().ok_or(Errno::ENOENT),
		Base::WorkingDir { to_parent } => to_parent,
	};

	let mut curr = task
		.get_user_ext()
		.expect("must be user process")
		.lock_cwd()
		.clone();

	for _ in 0..depth {
		curr = curr.parent_dir(&task)?;
	}

	Ok(curr)
}

pub fn lookup_entry(path: Path, task: &Arc<Task>) -> Result<VfsEntry, Errno> {
	let mut curr = VfsEntry::Dir(lookup_base_entry(path.base(), task)?);
	for comp in path.components() {
		let dir = curr.downcast_dir()?;

		curr = dir.lookup(&comp, task)?;
	}

	Ok(curr)
}

pub fn lookup_dir_entry(path: Path, task: &Arc<Task>) -> Result<Arc<VfsDirEntry>, Errno> {
	lookup_entry(path, task).and_then(|x| x.downcast_dir())
}

pub fn lookup_file_entry(path: Path, task: &Arc<Task>) -> Result<Arc<VfsFileEntry>, Errno> {
	lookup_entry(path, task).and_then(|x| x.downcast_file())
}
