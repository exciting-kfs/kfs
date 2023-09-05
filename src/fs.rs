pub mod ext2;
pub mod path;
pub mod syscall;
pub mod vfs;

pub mod devfs;
mod tmpfs;

use crate::driver::ide::device_number;
use crate::driver::ide::ide_id::{IdeId, NR_IDE_DEV};
use crate::driver::ide::partition::entry::EntryIndex;
use crate::driver::ide::partition::{get_partition_entry, PartitionType, NR_PRIMARY};
use crate::fs::vfs::PseudoFileSystem;
use crate::syscall::errno::Errno;
use alloc::{rc::Rc, sync::Arc, vec::Vec};
use tmpfs::TmpFs;

use self::ext2::Ext2;
use self::vfs::{DirInode, FileSystem, SuperBlock, VfsDirEntry, ROOT_DIR_ENTRY};

pub fn init() -> Result<(), Errno> {
	read_ide_dev();

	let (sb, inode) = TmpFs::mount()?;

	let name = Rc::new(Vec::new());
	let _ = ROOT_DIR_ENTRY.lock().insert(Arc::new_cyclic(|w| {
		VfsDirEntry::new(name, inode, w.clone(), sb, true)
	}));

	Ok(())
}

fn read_ide_dev() -> (Vec<Arc<dyn SuperBlock>>, Vec<Arc<dyn DirInode>>) {
	let mut sb: Vec<Arc<dyn SuperBlock>> = Vec::new();
	let mut dir_inode: Vec<Arc<dyn DirInode>> = Vec::new();
	for (id, ei) in (0..NR_IDE_DEV)
		.zip(0..NR_PRIMARY)
		.map(|(d, p)| unsafe { (IdeId::new_unchecked(d), EntryIndex::new_unchecked(p)) })
	{
		let dev = device_number(id, Some(ei));
		let maybe = get_partition_entry(id, ei);
		if let Some(entry) = maybe.get() {
			let res = match entry.partition_type {
				PartitionType::Linux => {
					drop(maybe);
					Some(Ext2::mount(dev))
				}
				_ => None,
			};

			if let Some(res) = res {
				let (s, d) = res.expect("ext2");
				sb.push(s);
				dir_inode.push(d);
			}
		}
	}

	(sb, dir_inode)
}
