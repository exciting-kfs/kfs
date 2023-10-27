use alloc::sync::Arc;

use crate::fs::sysfs::SysFs;
use crate::mm::user::verify::{verify_path, verify_string};
use crate::process::task::CURRENT;
use crate::syscall::errno::Errno;
use crate::{
	fs::{
		devfs::{partition::PartBorrow, DevFs},
		ext2::Ext2,
		path::Path,
		procfs::ProcFs,
		tmpfs::TmpFs,
		vfs::{lookup_entry_nofollow, MemoryFileSystem, PhysicalFileSystem, VfsDirEntry},
	},
	process::task::Task,
};

macro_rules! mount_arm {
	(MEMFS $blk:ident | $mount_point:ident | $task:ident | $fs:ty) => {{
		let (sb, inode) = <$fs>::mount()?;
		let new_dentry = $mount_point.mount(inode, sb, $task)?;
		<$fs>::finish_mount(&new_dentry);

		return Ok(0);
	}};

	(PHYFS $blk:ident | $mount_point:ident | $task:ident | $fs:ty) => {{
		let (sb, inode) = <$fs>::mount($blk?)?;
		_ = $mount_point.mount(inode, sb, $task)?;

		return Ok(0);
	}};
}

macro_rules! mount_fs {
	($blk:ident, $fs_name:ident, $mount_point:ident, $task:ident {$($typ:ident $name:literal => $fs:ty),* $(,)?}) => {
		match $fs_name {
			$(
				$name => mount_arm!($typ $blk | $mount_point | $task | $fs),
			)*
			_ => return Err(Errno::EINVAL),
		}
	};
}

fn do_mount(
	block_device: Result<PartBorrow, Errno>,
	fs_name: &[u8],
	mount_point_entry: Arc<VfsDirEntry>,
	task: &Arc<Task>,
) -> Result<usize, Errno> {
	mount_fs!(block_device, fs_name, mount_point_entry, task {
		MEMFS b"tmpfs" => TmpFs,
		MEMFS b"procfs" => ProcFs,
		MEMFS b"devfs" => DevFs,
		MEMFS b"sysfs" => SysFs,
		PHYFS b"ext2" => Ext2,
	})
}

pub fn sys_mount(dev_path: usize, mount_point: usize, fs_name: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let dev_path = verify_path(dev_path, current)?;
	let dev_path = Path::new(dev_path);
	let mount_point = verify_path(mount_point, current)?;
	let mount_point = Path::new(mount_point);
	let fs_name = verify_string(fs_name, current, 256)?;

	let entry = lookup_entry_nofollow(&mount_point, current).and_then(|x| x.downcast_dir())?;
	let block_device = lookup_entry_nofollow(&dev_path, current)
		.and_then(|x| x.downcast_block())
		.and_then(|x| x.get_device());

	do_mount(block_device, fs_name, entry, current)
}

pub fn sys_umount(path: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let path = verify_path(path, current)?;
	let path = Path::new(path);
	let entry = lookup_entry_nofollow(&path, current).and_then(|x| x.downcast_dir())?;

	entry.unmount(current).map(|_| 0)
}
