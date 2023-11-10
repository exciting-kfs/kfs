use crate::{
	fs::{
		path::Path,
		vfs::{lookup_entry_follow_except_last, VfsEntry},
	},
	mm::user::verify::{verify_buffer_mut, verify_path},
	process::task::CURRENT,
	syscall::errno::Errno,
};

pub fn sys_readlink(path: usize, buf: usize, bufsize: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);

	let buf = verify_buffer_mut(buf, bufsize, current)?;

	let symlink = lookup_entry_follow_except_last(&path, current).and_then(|ent| match ent {
		VfsEntry::SymLink(env) => Ok(env),
		_ => Err(Errno::EINVAL),
	})?;

	let target = symlink.target()?.to_buffer();

	let size = buf.len().min(target.len());

	buf[..size].copy_from_slice(&target[..size]);

	Ok(size)
}
