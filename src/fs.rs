pub mod ext2;
pub mod path;
pub mod syscall;
pub mod vfs;

mod devfs;
mod procfs;
mod tmpfs;

use alloc::{rc::Rc, sync::Arc, vec::Vec};

use crate::driver::dev_num::DevNum;
use crate::driver::ide::device_number;
use crate::driver::ide::ide_id::{IdeId, NR_IDE_DEV};
use crate::driver::ide::partition::entry::{EntryIndex, PartitionEntry};
use crate::driver::ide::partition::{get_partition_entry, PartitionType, NR_PRIMARY};

use crate::fs::vfs::FileSystem;
use crate::syscall::errno::Errno;

use ext2::Ext2;
use tmpfs::TmpFs;
use vfs::{DirInode, PhysicalFileSystem, SuperBlock, VfsDirEntry, ROOT_DIR_ENTRY};

pub use devfs::init as init_devfs;
pub use procfs::init as init_procfs;
pub use procfs::{change_cwd, create_fd_node, create_task_node, delete_fd_node, delete_task_node};

pub fn init() -> Result<(), Errno> {
	read_ide_dev();

	let (sb, inode) = TmpFs::mount()?;

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	Ok(())
}

fn read_ide_dev() -> Vec<(Arc<dyn SuperBlock>, Arc<dyn DirInode>)> {
	let mut filesystem = Vec::new();
	for (id, ei) in (0..NR_IDE_DEV)
		.zip(0..NR_PRIMARY)
		.map(|(d, p)| unsafe { (IdeId::new_unchecked(d), EntryIndex::new_unchecked(p)) })
	{
		let dev = device_number(id, Some(ei));
		let maybe = get_partition_entry(id, ei);

		if let Some(fs) = maybe.get().and_then(|entry| identify_fs(dev, entry)) {
			filesystem.push(fs);
		}
	}

	filesystem
}

fn identify_fs(
	dev: DevNum,
	entry: &PartitionEntry,
) -> Option<(Arc<dyn SuperBlock>, Arc<dyn DirInode>)> {
	match entry.partition_type {
		PartitionType::Linux => Ext2::mount(dev).ok(),
		_ => None,
	}
	.map(|(sb, root)| {
		let sb: Arc<dyn SuperBlock> = sb;
		let root: Arc<dyn DirInode> = root;
		(sb, root)
	})
}
