mod bgd;
pub mod dir;
pub mod file;
pub mod inode;
pub mod sb;

use core::{mem::size_of, sync::atomic::Ordering};

use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

use crate::{
	driver::{
		dev_num::DevNum,
		ide::{
			block::{Block, BlockSize},
			dma::wait_io::WaitIO,
			get_ide_controller,
			ide_id::IdeId,
			IdeController,
		},
		partition::{
			entry::{EntryIndex, PartitionEntry},
			table::get_partition_entry,
		},
	},
	fs::ext2::{
		inode::Inum,
		sb::{SuperBlock, SuperBlockInfo},
	},
	sync::{LockRW, Locked, LockedGuard},
	syscall::errno::Errno,
	RUN_TIME,
};

use self::{
	bgd::{BGD, BGDT},
	inode::DirInode,
};

use super::vfs;

const MAGIC: u16 = 0xef53; // TODO check this..

pub struct Ext2;

impl Ext2 {
	fn read_superblock<'a>(
		ide: &LockedGuard<'a, IdeController>,
		entry: &PartitionEntry,
	) -> Result<SuperBlockInfo, Errno> {
		let block_size = BlockSize::from_sector_count(1).unwrap();
		let mut mem = Block::new(block_size).map_err(|_| Errno::ENOMEM)?.into();
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
		let table_size = size_of::<BGD>() * sb.group_count(); // Max: 8MB

		for (idx, start) in (0..table_size).step_by(BlockSize::MAX_KB).enumerate() {
			// alloc block
			let block_size = match table_size - start > BlockSize::MAX_KB {
				true => BlockSize::BIGGEST,
				false => unsafe { BlockSize::new_unchecked(table_size - start) },
			};
			let mut mem = Block::new(block_size).map_err(|_| Errno::ENOMEM)?.into();

			// read sectors
			let buf = unsafe { mem.as_slice_mut(block_size.sector_count()) };
			let lba = sb
				.bgdt_lba(entry.begin())
				.block_size_add(BlockSize::BIGGEST, idx);
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
		let sb_data = Ext2::read_superblock(&ide, entry)?;
		let bgd_table = Ext2::read_bgd_table(&ide, entry, &sb_data)?;
		drop(ide);

		let sb = Arc::new(SuperBlock {
			ide_id: id,
			entry: maybe_entry,
			info: LockRW::new(sb_data),
			bgd_table: LockRW::new(bgd_table),
			inode_cache: Locked::new(BTreeMap::new()),
			wait_io: WaitIO::new(),
		});

		let inum = unsafe { Inum::new_unchecked(2) };
		let root = match RUN_TIME.load(Ordering::Relaxed) {
			true => sb.read_inode_dma(inum),
			false => sb.read_inode_pio(inum).map_err(|_| Errno::ENOMEM),
		}?
		.downcast_dir()
		.unwrap();

		Ok((sb, Arc::new(root)))
	}
}
