mod block_pool;
mod staged;

pub mod dir;
pub mod file;
pub mod inode;
pub mod sb;
pub mod symlink;

pub use block_pool::block::Block;

use core::{mem::size_of, sync::atomic::Ordering};

use alloc::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	vec::Vec,
};

use crate::{
	driver::{
		dev_num::DevNum,
		ide::{
			block::{Block as IdeBlock, BlockSize},
			device_number, get_ide_controller,
			ide_id::{IdeId, NR_IDE_DEV},
			partition::{
				entry::{EntryIndex, PartitionEntry},
				get_partition_entry, PartitionType, NR_PRIMARY,
			},
			IdeController,
		},
	},
	fs::ext2::sb::SuperBlock,
	process::get_init_task,
	sync::{LocalLocked, LockRW, Locked, LockedGuard},
	syscall::errno::Errno,
	RUN_TIME,
};

use self::{
	block_pool::BlockPool,
	dir::dir_inode::DirInode,
	inode::inum::Inum,
	sb::{
		bgd::{BGD, BGDT},
		info::SuperBlockInfo,
	},
};

use super::vfs::{self, Permission, PhysicalFileSystem, ROOT_DIR_ENTRY};

const MAGIC: u16 = 0xef53; // TODO check this..

pub struct Ext2;

impl Ext2 {
	fn read_superblock<'a>(
		ide: &LockedGuard<'a, IdeController>,
		entry: &PartitionEntry,
	) -> Result<SuperBlockInfo, Errno> {
		let block_size = BlockSize::from_sector_count(1).unwrap();
		let mut mem = IdeBlock::new(block_size).map_err(|_| Errno::ENOMEM)?.into();
		let sector = unsafe { mem.as_slice_mut(1) };

		// The superblock is always located at byte offset 1024 from the begining of the partition.
		let lba = entry.begin() + 2;
		ide.ata.read_sectors(lba, sector);

		Ok(unsafe { mem.into::<SuperBlockInfo>().as_one().clone() })
	}

	fn read_bgd_table<'a>(
		ide: &LockedGuard<'a, IdeController>,
		entry: &PartitionEntry,
		sb: &SuperBlockInfo,
	) -> Result<BGDT, Errno> {
		let mut v = Vec::new();
		let table_size = sb.bgdt_size();
		let bgdt_lba = sb.bgdt_lba(entry.begin());

		for (idx, start) in (0..table_size).step_by(BlockSize::MAX_BYTE).enumerate() {
			// alloc block
			let block_size = match table_size - start > BlockSize::MAX_BYTE {
				true => BlockSize::BIGGEST,
				false => unsafe { BlockSize::new_unchecked(table_size - start) },
			};
			let mut mem = IdeBlock::new(block_size).map_err(|_| Errno::ENOMEM)?.into();

			// read sectors
			let buf = unsafe { mem.as_slice_mut(block_size.sector_count()) };
			let lba = bgdt_lba.block_size_add(BlockSize::BIGGEST, idx);
			ide.ata.read_sectors(lba, buf);

			// store result
			let count = block_size.as_bytes() / size_of::<BGD>();
			let bgd = unsafe { mem.into::<[BGD]>().into_box_slice(count) };
			v.push(bgd);
		}
		Ok(BGDT::new(v).expect("ext2 always has BGDT"))
	}
}

impl vfs::PhysicalFileSystem<SuperBlock, DirInode> for Ext2 {
	fn mount(info: DevNum) -> Result<(Arc<SuperBlock>, Arc<DirInode>), Errno> {
		let id = IdeId::from_devnum(&info).ok_or(Errno::EINVAL)?;
		let ei = EntryIndex::from_devnum(&info).ok_or(Errno::EINVAL)?;
		let maybe_entry = get_partition_entry(id, ei);
		let entry = maybe_entry.get().ok_or(Errno::EINVAL)?;

		let ide = get_ide_controller(id);
		let mut sb_info = Ext2::read_superblock(&ide, entry)?;
		let bgd_table = Ext2::read_bgd_table(&ide, entry, &sb_info)?;
		let block_pool = Arc::new(BlockPool::new(id, maybe_entry, &sb_info));
		drop(ide);

		sb_info.edit_for_mount();

		let sb = Arc::new(SuperBlock {
			info: LockRW::new(sb_info),
			bgd_table: LocalLocked::new(bgd_table),
			inode_cache: Locked::new(BTreeMap::new()),
			block_pool,
			dirty_icache: Locked::new(BTreeSet::new()),
		});

		// TEST: dump bgd table
		// {
		// 	let iter = sb.bgd_table.iter(sb.block_size());

		// 	for slice in iter {
		// 		pr_debug!("{:?}", slice);
		// 	}
		// }

		let inum = unsafe { Inum::new_unchecked(2) };
		let root = match RUN_TIME.load(Ordering::Relaxed) {
			true => sb.read_inode_dma(inum),
			false => sb.read_inode_pio(inum).map_err(|_| Errno::ENOMEM),
		}
		.and_then(|inode| {
			inode.load_bid().map_err(|_| Errno::ENOMEM)?;
			Ok(inode)
		})?
		.downcast_dir()
		.unwrap();

		Ok((sb, Arc::new(root)))
	}
}

pub fn init() -> Result<(), Errno> {
	let mut root = ROOT_DIR_ENTRY.lock();
	let root = root.as_mut().unwrap();

	let v = read_ide_dev();

	for (i, (sb, inode)) in v.into_iter().enumerate() {
		let mut file_name = Vec::from_iter(b"e".iter().map(|e| *e));
		file_name.push(b'0' + i as u8);

		let entry = root.mkdir(
			&file_name,
			Permission::from_bits_truncate(0o666),
			&get_init_task(),
		)?;

		entry.mount(inode, sb, &get_init_task())?;
	}
	Ok(())
}

fn read_ide_dev() -> Vec<(Arc<dyn vfs::SuperBlock>, Arc<dyn vfs::DirInode>)> {
	let mut filesystem = Vec::new();

	for id in (0..NR_IDE_DEV).map(|d| unsafe { IdeId::new_unchecked(d) }) {
		for ei in (0..NR_PRIMARY).map(|e| unsafe { EntryIndex::new_unchecked(e) }) {
			let dev = device_number(id, Some(ei));
			let maybe = get_partition_entry(id, ei);

			if let Some(fs) = maybe.get().and_then(|entry| identify_fs(dev, entry)) {
				filesystem.push(fs);
			}
		}
	}

	filesystem
}

fn identify_fs(
	dev: DevNum,
	entry: &PartitionEntry,
) -> Option<(Arc<dyn vfs::SuperBlock>, Arc<dyn vfs::DirInode>)> {
	match entry.partition_type {
		PartitionType::Linux => Ext2::mount(dev).ok(),
		_ => None,
	}
	.map(|(sb, root)| {
		let sb: Arc<dyn vfs::SuperBlock> = sb;
		let root: Arc<dyn vfs::DirInode> = root;
		(sb, root)
	})
}

fn oom_handler() {
	panic!("OOM");
}
