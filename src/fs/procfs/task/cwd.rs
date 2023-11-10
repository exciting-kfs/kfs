use alloc::sync::Arc;

use crate::fs::path::{format_path, Path};
use crate::fs::procfs::{PROCFS_ROOT_DIR, PROCFS_ROOT_DIR_ENTRY};
use crate::fs::vfs::Entry;
use crate::fs::{tmpfs::TmpSymLink, vfs::lookup_entry_at_follow};
use crate::{process::task::Task, syscall::errno::Errno};

pub fn change_cwd(task: &Arc<Task>) -> Result<(), Errno> {
	let procfs = unsafe { PROCFS_ROOT_DIR.assume_init_ref() };

	let dir = procfs
		.get_task_inode(&task.get_pid())
		.ok_or(Errno::ENOENT)?;

	let cwd = task
		.get_user_ext()
		.ok_or(Errno::ENOENT)
		.and_then(|ext| ext.lock_cwd().get_abs_path())
		.unwrap_or_else(|_| Path::new_root());

	dir.lock().cwd = Arc::new(TmpSymLink::new(cwd));

	if let Some(ent) = &*PROCFS_ROOT_DIR_ENTRY.lock() {
		let ent = lookup_entry_at_follow(
			ent.clone(),
			&format_path!("{}", task.get_pid().as_raw()),
			task,
		)
		.and_then(|ent| ent.downcast_dir())?;
		ent.remove_child_force(b"cwd");
	}

	Ok(())
}
