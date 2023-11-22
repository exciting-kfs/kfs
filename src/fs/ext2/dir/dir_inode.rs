use core::{
	mem::{size_of, transmute},
	ptr::copy_nonoverlapping,
};

use alloc::{boxed::Box, collections::VecDeque, sync::Arc};

use crate::fs::{ext2::LINK_MAX, vfs::Entry};
use crate::{
	fs::{
		ext2::{
			dir,
			file::FileInode,
			inode::{self, inum::Inum, Inode, IterBlockError},
			sb::SuperBlock,
			staged::Staged,
			symlink::SymLinkInode,
			Block,
		},
		vfs::{self, FileType, Permission},
	},
	handle_iterblock_error,
	sync::LockRW,
	syscall::errno::Errno,
	trace_feature,
};

use super::{dir_file::DirFile, record::Record, Dirent, DirentMut};

#[derive(Clone)]
pub struct DirInode(Arc<LockRW<Inode>>);

impl DirInode {
	const MAX_NAME_LEN: usize = 255;

	pub fn new_shared(
		parent_inum: Inum,
		self_inum: Inum,
		perm: Permission,
		block: &Arc<LockRW<Block>>,
		sb: &Arc<SuperBlock>,
	) -> Arc<Self> {
		Self::init_dirent(parent_inum, self_inum, block);

		let bid = block.read_lock().id();
		let inode = Inode::new_dir(self_inum, sb, perm, bid);
		let inode = Arc::new(LockRW::new(inode));

		sb.inode_cache.lock().insert(self_inum, inode.clone());
		inode.read_lock().dirty();

		Arc::new(Self(inode))
	}

	fn init_dirent(parent_inum: Inum, self_inum: Inum, block: &Arc<LockRW<Block>>) {
		let block_size = block.read_lock().size();

		let s_cap = Record::capacity_need(b".");
		let s_rec = Record::new_dir_with_name(self_inum, s_cap, 1, b".\0\0\0");

		let p_cap = block_size as u16 - s_cap;
		let p_rec = Record::new_dir_with_name(parent_inum, p_cap, 2, b"..\0\0");
		let records = s_rec.chain(p_rec);

		block
			.as_slice_mut()
			.iter_mut()
			.zip(records)
			.for_each(|(b, r)| {
				*b = r;
			});
	}

	pub fn from_inode(inode: Arc<LockRW<Inode>>) -> Self {
		Self(inode)
	}

	#[inline]
	pub(super) fn inner(&self) -> &Arc<LockRW<Inode>> {
		&self.0
	}

	fn is_empty(&self) -> bool {
		if self.inner().info().links_count == 2 {
			true
		} else {
			false
		}
	}

	fn super_block(&self) -> Arc<SuperBlock> {
		self.0.super_block()
	}

	fn ensure_space(&self, name: &[u8]) -> Result<(DirentMut, inode::ChunkMut), Errno> {
		let mut iter = dir::Iter::new(self, 0);

		loop {
			let cursor = iter.cursor();
			let chunk = iter.next_block();

			if let Ok(dirent) = chunk {
				let record = dirent.get_record();
				if dirent.get_record().is_allocatable(name) {
					let space = self.point_space(cursor, &record);
					iter.rewind();

					return unsafe { iter.next_mut_block_unchecked().map(|ent| (ent, space)) };
				}
			} else {
				handle_iterblock_error!(chunk.unwrap_err());
			}
		}

		iter.rewind();
		let space = self.alloc_space(&mut iter)?;

		unsafe {
			iter.next_mut_block_unchecked()
				.map(|dirent| (dirent, space))
		}
	}

	fn point_space(&self, cursor: usize, record: &Record) -> inode::ChunkMut {
		let (cursor, remain) = { (cursor + record.len(), record.remain_space()) };

		inode::Iter::new(self.inner().clone(), cursor)
			.next_mut_block(remain)
			.unwrap()
	}

	fn alloc_space(&self, dir_iter: &mut dir::Iter) -> Result<inode::ChunkMut, Errno> {
		let inode = self.inner();
		let block_size = inode.read_lock().block_size();

		let mut inode_iter = inode::Iter::write_end(inode.clone());

		let chunk = inode_iter.next_mut_block(block_size)?;

		let mut dirent = unsafe { dir_iter.next_mut_block_unchecked()? };
		dirent.get_record().capacity_add(block_size);
		dir_iter.rewind();

		Ok(chunk)
	}

	fn write_dirent_staged(
		&self,
		name: &[u8],
		file_type: FileType,
	) -> Result<Staged<Inum, ()>, Errno> {
		if name.len() > Self::MAX_NAME_LEN {
			return Err(Errno::ENAMETOOLONG);
		}

		let (mut dirent, space) = self.ensure_space(name)?;

		let name = name.to_vec();
		let write_record = Staged::func(move |inum: Inum| {
			let name_len = name.len() as u8;
			let rec_len = space.len() as u16;

			let record = match file_type {
				FileType::Directory => Record::new_dir(inum, name_len, rec_len),
				FileType::SymLink => Record::new_symlink(inum, name_len, rec_len),
				FileType::Socket => todo!(),
				_ => Record::new_file(inum, name_len, rec_len),
			};

			write_dirent(&mut space.slice_mut(), &record, &name);
			dirent.get_record().capacity_sub(space.len());
		});

		Ok(write_record)
	}

	fn find_dirent(&self, name: &[u8]) -> Result<dir::Iter, Errno> {
		let mut iter = dir::Iter::new(self, 0);

		loop {
			let chunk = iter.next_block();

			if let Ok(dirent) = chunk {
				if dirent.get_name().eq(name) {
					iter.rewind();
					return Ok(iter);
				}
			} else {
				handle_iterblock_error!(chunk.unwrap_err())
			}
		}

		Err(Errno::ENOENT)
	}

	fn get_dirent_with_prev(&self, name: &[u8]) -> Result<(DirentMut, Dirent), Errno> {
		let mut iter = self.find_dirent(name)?;

		let curr = unsafe { iter.next_block_unchecked()? };

		iter.rewind();
		iter.rewind();

		let prev = unsafe { iter.next_mut_block_unchecked()? };

		Ok((prev, curr))
	}

	fn remove_dirent_staged<F>(&self, name: &[u8], check_type: F) -> Result<(usize, Staged), Errno>
	where
		F: FnOnce(FileType) -> Result<(), Errno>,
	{
		let (mut prev, curr) = self.get_dirent_with_prev(name)?;

		let curr_record = curr.get_record();
		let ino = curr_record.ino as usize;
		let rec_len = curr_record.capacity();
		check_type(curr_record.file_type)?;
		drop(curr_record);

		Ok((
			ino,
			Staged::new(move |_| {
				let mut prev_record = prev.get_record();
				prev_record.capacity_add(rec_len)
			}),
		))
	}

	fn new_child_with_space(
		&self,
		name: &[u8],
		file_type: FileType,
	) -> Result<(Inum, Arc<LockRW<Block>>), Errno> {
		let sb = self.super_block();

		let child_inum = sb.alloc_inum_staged()?;
		let dirent = self.write_dirent_staged(name, file_type)?;

		let block = sb.alloc_blocks(1)?[0].clone();

		let child_inum = child_inum.commit(());
		dirent.commit(child_inum);

		Ok((child_inum, block))
	}

	fn new_child(&self, name: &[u8], file_type: FileType) -> Result<Inum, Errno> {
		let sb = self.super_block();

		let child_inum = sb.alloc_inum_staged()?;
		let dirent = self.write_dirent_staged(name, file_type)?;

		let child_inum = child_inum.commit(());
		dirent.commit(child_inum);
		Ok(child_inum)
	}

	fn remove_child(&self, child: &Arc<LockRW<Inode>>) -> Result<(), Errno> {
		child.load_bid()?;

		let sb = self.super_block();
		let inum = child.read_lock().inum();
		let inum_staged = sb.dealloc_inum_staged(inum)?;

		let mut blocks = VecDeque::new();
		{
			let data = child.data_read();
			let mut bids = data.block_id().into_iter();
			while let Some(bid) = bids.next() {
				let staged = sb.dealloc_block_staged(bid)?;
				blocks.push_back(staged);
			}
		}

		let info = child.info().clone_for_delete();

		child.data_write().clear();
		child.info_mut().write(&info);

		inum_staged.commit(());
		blocks.into_iter().for_each(|b| b.commit(()));

		Ok(())
	}

	fn inode_to_vfs(&self, inode: Arc<LockRW<Inode>>, file_type: FileType) -> vfs::VfsInode {
		match file_type {
			FileType::Directory => vfs::VfsInode::Dir(Arc::new(DirInode::from_inode(inode))),
			FileType::SymLink => vfs::VfsInode::SymLink(Arc::new(SymLinkInode::from_inode(inode))),
			FileType::Socket => todo!(),
			_ => vfs::VfsInode::File(Arc::new(FileInode::from_inode(inode))),
		}
	}

	fn ino_to_inode(&self, ino: usize) -> Result<Arc<LockRW<Inode>>, Errno> {
		Inum::new(ino)
			.ok_or(Errno::EINVAL)
			.and_then(|inum| self.super_block().read_inode_dma(inum))
	}
}

impl vfs::Inode for DirInode {
	fn stat(&self) -> Result<vfs::Statx, Errno> {
		Ok(self.inner().read_lock().stat())
	}

	fn chmod(&self, perm: vfs::Permission) -> Result<(), Errno> {
		self.inner().info_mut().chmod(perm)
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		self.inner().info_mut().chown(owner, group)
	}
}

#[allow(unused)]
impl vfs::DirInode for DirInode {
	fn open(&self) -> Result<Box<dyn vfs::DirHandle>, Errno> {
		self.inner().load_bid().map_err(|_| Errno::ENOMEM)?;

		// TEST: dump sb
		// {
		// 	let sb = self.super_block();
		// 	for bid in sb.sb_backup_bid() {
		// 		let block = sb.block_pool.get_or_load(bid)?;
		// 		let ptr = block.read_lock().as_slice_ref().as_ptr();

		// 		unsafe { print_memory(ptr, block.read_lock().size()) };
		// 	}
		// }

		Ok(Box::new(DirFile::new(self)))
	}

	fn lookup(&self, name: &[u8]) -> Result<vfs::VfsInode, Errno> {
		let mut dirent = self.find_dirent(name)?;
		let chunk = unsafe { dirent.next_block_unchecked()? };

		let (ino, file_type) = {
			let record = chunk.get_record();
			(record.ino as usize, record.file_type)
		};

		let inum = unsafe { Inum::new_unchecked(ino) };
		let inode = self.super_block().read_inode_dma(inum)?;
		inode.load_bid()?;

		Ok(self.inode_to_vfs(inode, file_type))
	}

	fn symlink(&self, target: &[u8], name: &[u8]) -> Result<Arc<dyn vfs::SymLinkInode>, Errno> {
		if let Ok(_) = self.find_dirent(name) {
			return Err(Errno::EEXIST);
		}

		let sb = self.super_block();
		let (child, block_size) = if target.len() > 60 {
			let (inum, block) = self.new_child_with_space(name, FileType::SymLink)?;
			let block_size = block.read_lock().size();
			let child = SymLinkInode::with_block(target, inum, &block, &sb);
			(child, block_size)
		} else {
			let inum = self.new_child(name, FileType::SymLink)?;
			let child = SymLinkInode::new(target, inum, &sb);
			(child, 0)
		};

		{
			let mut info = child.inner().info_mut();
			info.inc_blocks(block_size);
			info.set_size(target.len());
		}

		self.inner().info_mut().links_count += 1;
		vfs::SuperBlock::sync(sb.as_ref());

		Ok(child)
	}

	fn mkdir(&self, name: &[u8], perm: vfs::Permission) -> Result<Arc<dyn vfs::DirInode>, Errno> {
		if let Ok(_) = self.find_dirent(name) {
			return Err(Errno::EEXIST);
		}

		let (c_inum, block) = self.new_child_with_space(name, FileType::Directory)?;
		let p_inum = self.inner().read_lock().inum();

		let sb = self.super_block();
		let child = DirInode::new_shared(p_inum, c_inum, perm, &block, &sb);

		{
			let block_size = block.read_lock().size();
			let mut info = child.inner().info_mut();
			info.set_size(block_size);
			info.inc_blocks(block_size);

			self.inner().info_mut().links_count += 1;
		}

		vfs::SuperBlock::sync(sb.as_ref());

		Ok(child)
	}

	fn create(&self, name: &[u8], perm: vfs::Permission) -> Result<Arc<dyn vfs::FileInode>, Errno> {
		if let Ok(_) = self.find_dirent(name) {
			return Err(Errno::EEXIST);
		}

		let sb = self.super_block();
		let inum = self.new_child(name, FileType::Regular)?;
		let child = FileInode::new_shared(&sb, inum, perm);

		self.inner().info_mut().links_count += 1;
		vfs::SuperBlock::sync(sb.as_ref());

		Ok(child)
	}

	fn rmdir(&self, name: &[u8]) -> Result<(), Errno> {
		let (ino, mut record) = self.remove_dirent_staged(name, |file_type| match file_type {
			FileType::Directory => Ok(()),
			_ => Err(Errno::ENOTDIR),
		})?;

		let inum = unsafe { Inum::new_unchecked(ino) };
		let sb = self.super_block();
		let child = sb.read_inode_dma(inum)?;
		let dir = child.clone().downcast_dir().unwrap();

		if !dir.is_empty() {
			return Err(Errno::ENOTEMPTY);
		}

		self.remove_child(&child)?;

		record.commit(());
		self.0.info_mut().links_count -= 1;

		vfs::SuperBlock::sync(self.super_block().as_ref());
		Ok(())
	}

	fn unlink(&self, name: &[u8]) -> Result<(), Errno> {
		let (ino, mut record) = self.remove_dirent_staged(name, |_| Ok(()))?;

		let inum = unsafe { Inum::new_unchecked(ino) };
		let sb = self.super_block();
		let child = sb.read_inode_dma(inum)?;

		{
			if child.info().links_count == 1 {
				self.remove_child(&child)?;
			} else {
				child.info_mut().links_count -= 1;
			}
		}

		record.commit(());

		vfs::SuperBlock::sync(self.super_block().as_ref());
		Ok(())
	}

	fn link(&self, src: &vfs::VfsEntry, link_name: &[u8]) -> Result<vfs::VfsInode, Errno> {
		trace_feature!( "ext2-hard_link"
			"HARD LINK from: {:?} to {:?}",
			String::from_utf8(target.get_name().to_vec()),
			String::from_utf8(link_name.to_vec())
		);

		let stat = src.statx()?;
		let file_type = stat.get_type();

		let inum = Inum::new(stat.ino as usize).ok_or(Errno::EINVAL)?;
		let inode = self.super_block().read_inode_dma(inum)?;

		if inode.info().links_count >= LINK_MAX {
			return Err(Errno::EMLINK);
		}

		let dirent = self.write_dirent_staged(link_name, file_type)?;

		dirent.commit(inum);

		{
			let mut info = inode.info_mut();
			info.links_count += 1;
		}

		Ok(self.inode_to_vfs(inode, file_type))
	}

	fn overwrite(&self, src: &vfs::VfsEntry, link_name: &[u8]) -> Result<vfs::VfsInode, Errno> {
		let sb = self.super_block();
		let stat = src.statx()?;
		let file_type = stat.get_type();

		let mut iter = self.find_dirent(link_name)?;
		let mut dirent = unsafe { iter.next_mut_block_unchecked() }?;
		let mut record = dirent.get_record();

		let old_child = self.ino_to_inode(record.ino as usize)?;
		let new_child = self.ino_to_inode(stat.ino as usize)?;

		if new_child.info().links_count >= LINK_MAX {
			return Err(Errno::EMLINK);
		}

		trace_feature!(
			"ext2-overwrite",
			"OVERWRITE: src: ino:{}, record: name: {:?} ino: {}",
			stat.ino,
			alloc::string::String::from_utf8(link_name.to_vec()),
			record.ino
		);

		let is_unique = old_child.info().is_unique();
		match (file_type, is_unique) {
			(FileType::Directory | FileType::Regular | FileType::SymLink, true) => {
				self.remove_child(&old_child)
			}
			(FileType::Regular | FileType::SymLink, false) => {
				old_child.info_mut().links_count -= 1;
				Ok(())
			}
			(FileType::Directory, false) => Err(Errno::ENOTEMPTY),
			_ => Err(Errno::EPERM),
		}?;

		record.ino = stat.ino as u32;
		record.file_type = file_type;
		new_child.info_mut().links_count += 1;

		Ok(self.inode_to_vfs(new_child, file_type))
	}
}

fn write_dirent(buf: &mut [u8], record: &Record, name: &[u8]) {
	trace_feature!("ext2-mkdir" | "ext2-create" "record: {:?}, name: {:?}", record, alloc::string::String::from_utf8(name.to_vec()));

	let record: &[u8; size_of::<Record>()] = unsafe { transmute(record) };

	unsafe {
		let ptr = buf.as_mut_ptr();
		copy_nonoverlapping(record.as_ptr(), ptr, record.len());

		let ptr = ptr.offset(record.len() as isize);
		copy_nonoverlapping(name.as_ptr(), ptr, name.len());
	}
}
