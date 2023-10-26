mod block_pool;
mod constant;
mod staged;

pub mod dir;
pub mod file;
pub mod inode;
pub mod sb;
pub mod symlink;

pub use block_pool::block::Block;

use core::{
	mem::{size_of, transmute, MaybeUninit},
	ptr::copy_nonoverlapping,
	sync::atomic::Ordering,
};

use alloc::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
	vec::Vec,
};

use crate::{
	driver::partition::{BlockId, Partition},
	fs::ext2::sb::SuperBlock,
	mm::util::next_align,
	pr_debug,
	sync::{LocalLocked, LockRW, Locked},
	syscall::errno::Errno,
	trace_feature, RUN_TIME,
};

use self::{
	block_pool::BlockPool,
	inode::inum::Inum,
	sb::{
		bgd::{BGD, BGDT},
		info::SuperBlockInfo,
	},
};

use super::{
	devfs::partition::PartBorrow,
	vfs::{self, FileSystem},
};

const MAGIC: u16 = 0xef53;

static SB_POOL: Locked<BTreeMap<Vec<u8>, Arc<SuperBlock>>> = Locked::new(BTreeMap::new());

pub struct Ext2;

impl Ext2 {
	fn read_superblock<'a>(block_dev: &Arc<Partition>) -> Result<SuperBlockInfo, Errno> {
		let mut sb: MaybeUninit<[u8; size_of::<SuperBlockInfo>()]> = MaybeUninit::uninit();

		unsafe {
			let bid = BlockId::new_unchecked(0);
			let block = match RUN_TIME.load(Ordering::Relaxed) {
				true => block_dev.load(bid)?.into(),
				false => block_dev.load_pio(bid)?.into(),
			};

			let slice: &[u8] = block.as_slice_ref(1024 + size_of::<SuperBlockInfo>());

			// The superblock is always located at byte offset 1024 from the begining of the partition.
			copy_nonoverlapping(
				slice.as_ptr().offset(1024),
				sb.as_mut_ptr().cast(),
				size_of::<SuperBlockInfo>(),
			);

			Ok(transmute(sb))
		}
	}

	fn read_bgd_table<'a>(block_dev: &Arc<Partition>, sb: &SuperBlockInfo) -> Result<BGDT, Errno> {
		let mut v = Vec::new();
		let block_size = block_dev.block_size().as_bytes();
		let table_size = sb.bgdt_size();
		let begin_bid = sb.bgdt_bid();
		let count = next_align(table_size, block_size) / block_size;

		for bid in (0..count).map(|i| unsafe { BlockId::new_unchecked(begin_bid.inner() + i) }) {
			let block = match RUN_TIME.load(Ordering::Relaxed) {
				true => block_dev.load(bid)?,
				false => block_dev.load_pio(bid)?,
			};

			let count = block_size / size_of::<BGD>();
			let bgd = unsafe { block.into::<[BGD]>().into_box_slice(count) };
			v.push(bgd);
		}

		Ok(BGDT::new(v).expect("ext2 always has BGDT"))
	}
}

impl FileSystem for Ext2 {
	fn unmount(&self, sb: &Arc<dyn vfs::SuperBlock>) -> Result<(), Errno> {
		sb.unmount()?;

		trace_feature!("ext2-unmount", "SuperBlock unmounted: {:x?}", sb.id());
		SB_POOL.lock().remove(&sb.id());

		Ok(())
	}
}

impl vfs::PhysicalFileSystem for Ext2 {
	fn mount(
		block_dev: PartBorrow,
	) -> Result<(Arc<dyn vfs::SuperBlock>, Arc<dyn vfs::DirInode>), Errno> {
		let mut sb_info = Ext2::read_superblock(&block_dev)?;
		trace_feature!("ext2-mount", "sb: {:?}", sb_info);

		if sb_info.magic() != MAGIC {
			return Err(Errno::EINVAL);
		}

		block_dev.init(sb_info.block_size());

		let bgd_table = Ext2::read_bgd_table(&block_dev, &sb_info)?;
		let block_pool = Arc::new(BlockPool::new(block_dev));

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

		let ret = match RUN_TIME.load(Ordering::Relaxed) {
			true => sb.read_inode_dma(inum),
			false => sb.read_inode_pio(inum).map_err(|_| Errno::ENOMEM),
		}
		.and_then(|inode| {
			inode.load_bid()?;
			Ok(inode)
		})?
		.downcast_dir()
		.map(|root| (sb, Arc::new(root)))
		.map_err(|_| Errno::EINVAL);

		if let Ok((sb, _)) = ret.as_ref() {
			let uuid = sb.info.read_lock().uuid().to_vec();
			trace_feature!(
				"ext2-mount",
				"SuperBlock mounted: {:x?}",
				vfs::SuperBlock::id(sb.as_ref())
			);
			SB_POOL.lock().insert(uuid, sb.clone());
		}

		let (sb, inode) = ret?;

		Ok((sb, inode))
	}
}

pub fn oom_handler() {
	let map = SB_POOL.lock();

	for (_, sb) in map.iter() {
		sb.block_pool.handle_overflow(0);
	}
}

pub fn clean_up() -> Result<(), Errno> {
	pr_debug!("ext2: cleanup called");
	let mut pool = SB_POOL.lock();
	while let Some((_, sb)) = pool.pop_first() {
		drop(pool);
		let sb: Arc<dyn vfs::SuperBlock> = sb;
		sb.unmount()?;
		pool = SB_POOL.lock();
	}

	Ok(())
}
