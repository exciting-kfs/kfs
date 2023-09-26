use core::mem::size_of;
use core::ptr::addr_of_mut;

use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{boxed::Box, collections::BTreeMap};

use super::path::Path;
use super::vfs::{
	DirHandle, DirInode, FileHandle, FileInode, FileSystem, IOFlag, Ident, RawStat, SuperBlock,
	SymLinkInode, TimeSpec, VfsInode, Whence,
};
use crate::fs::vfs::{KfsDirent, Permission};
use crate::mm::util::next_align;
use crate::process::task::CURRENT;
use crate::sync::Locked;
use crate::syscall::errno::Errno;

pub struct TmpFs;

impl FileSystem<TmpSb, Locked<TmpDirInode>> for TmpFs {
	fn mount() -> Result<(Arc<TmpSb>, Arc<Locked<TmpDirInode>>), Errno> {
		Ok((
			Arc::new(TmpSb),
			TmpDirInode::new_shared(Permission::from_bits_truncate(0o755), 0, 0),
		))
	}
}

pub struct TmpSb;

impl SuperBlock for TmpSb {
	fn sync(&self) {}
}

#[derive(Clone)]
pub enum TmpInode {
	Dir(Arc<Locked<TmpDirInode>>),
	File(Arc<TmpFileInode>),
	SymLink(Arc<TmpSymLink>),
}

impl Into<VfsInode> for TmpInode {
	fn into(self) -> VfsInode {
		match self {
			TmpInode::Dir(d) => VfsInode::Dir(d),
			TmpInode::File(f) => VfsInode::File(f),
			TmpInode::SymLink(s) => VfsInode::SymLink(s),
		}
	}
}

pub struct TmpFileInode {
	data: Arc<Locked<Vec<u8>>>,
	perm: Locked<Permission>,
	owner: Locked<usize>,
	group: Locked<usize>,
}

impl TmpFileInode {
	pub fn new(perm: Permission, owner: usize, group: usize) -> Arc<Self> {
		Arc::new(Self {
			data: Arc::new(Locked::new(Vec::new())),
			perm: Locked::new(perm),
			owner: Locked::new(owner),
			group: Locked::new(group),
		})
	}
}

impl FileInode for TmpFileInode {
	fn open(&self) -> Box<dyn FileHandle> {
		TmpFile::new(self.data.clone())
	}

	fn truncate(&self, length: isize) -> Result<(), Errno> {
		if length < 0 {
			return Err(Errno::EINVAL);
		}

		let mut data = self.data.lock();
		data.resize(length as usize, 0);

		Ok(())
	}

	fn stat(&self) -> Result<RawStat, Errno> {
		Ok(RawStat {
			perm: self.perm.lock().bits(),
			uid: *self.owner.lock(),
			gid: *self.group.lock(),
			size: self.data.lock().len() as isize,
			file_type: 1,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		*self.owner.lock() = owner;
		*self.group.lock() = group;

		Ok(())
	}

	fn chmod(&self, perm: Permission) -> Result<(), Errno> {
		*self.perm.lock() = perm;

		Ok(())
	}
}

pub struct TmpFile {
	data: Arc<Locked<Vec<u8>>>,
	cursor: usize,
}

impl TmpFile {
	pub fn new(data: Arc<Locked<Vec<u8>>>) -> Box<Locked<Self>> {
		Box::new(Locked::new(Self { data, cursor: 0 }))
	}
}

impl FileHandle for Locked<TmpFile> {
	fn read(&self, buf: &mut [u8], _io_flags: IOFlag) -> Result<usize, Errno> {
		let mut this = self.lock();

		let size = {
			let data = this.data.lock();

			if data.len() <= this.cursor {
				return Ok(0);
			}

			let source = &data[this.cursor..];
			let size = source.len().min(buf.len());

			buf.copy_from_slice(&source[..size]);

			size
		};

		this.cursor += size;

		Ok(size)
	}

	fn write(&self, buf: &[u8], _io_flags: IOFlag) -> Result<usize, Errno> {
		let mut this = self.lock();

		let new_cursor = {
			let mut cursor = this.cursor;

			let mut data = this.data.lock();

			if data.len() < cursor {
				cursor = data.len();
			}

			let remain_space = data.len() - cursor;

			let (l, r) = match buf.len() < remain_space {
				true => (buf, &[] as &[u8]),
				false => buf.split_at(remain_space),
			};

			if remain_space != 0 {
				data[cursor..(cursor + l.len())].copy_from_slice(l);
			}

			data.extend_from_slice(r);

			cursor + buf.len()
		};

		this.cursor = new_cursor;

		Ok(buf.len())
	}

	fn lseek(&self, offset: isize, whence: Whence) -> Result<usize, Errno> {
		let mut this = self.lock();

		let new_cursor = {
			let data = this.data.lock();

			let new_cursor = match whence {
				Whence::Begin => offset,
				Whence::End => data.len() as isize + offset,
				Whence::Current => this.cursor as isize + offset,
			};

			if new_cursor < 0 || (data.len() as isize) < new_cursor {
				return Err(Errno::EINVAL);
			}

			new_cursor
		};

		this.cursor = new_cursor as usize;

		Ok(new_cursor as usize)
	}
}

pub struct TmpDirInode {
	sub_files: BTreeMap<Ident, TmpInode>,
	perm: Permission,
	owner: usize,
	group: usize,
}

impl TmpDirInode {
	pub fn new(perm: Permission, owner: usize, group: usize) -> Self {
		Self {
			sub_files: BTreeMap::new(),
			perm,
			owner,
			group,
		}
	}

	pub fn new_shared(perm: Permission, owner: usize, group: usize) -> Arc<Locked<Self>> {
		Arc::new(Locked::new(Self::new(perm, owner, group)))
	}

	fn is_empty(&self) -> bool {
		self.sub_files.is_empty()
	}
}

impl DirInode for Locked<TmpDirInode> {
	fn open(&self) -> Box<dyn DirHandle> {
		let this = self.lock();

		let mut v: Vec<(u8, Vec<u8>)> = Vec::new();

		for (name, inode) in this.sub_files.iter() {
			let kind = match inode {
				TmpInode::Dir(_) => 2,
				TmpInode::File(_) => 1,
				TmpInode::SymLink(_) => 7,
			};

			v.push((kind, name.to_vec()))
		}

		v.push((2, b".".to_vec()));
		v.push((2, b"..".to_vec()));

		Box::new(TmpDir::new(v))
	}

	fn stat(&self) -> Result<RawStat, Errno> {
		let this = self.lock();

		Ok(RawStat {
			perm: this.perm.bits(),
			uid: this.owner,
			gid: this.group,
			size: this.sub_files.len() as isize,
			file_type: 2,
			access_time: TimeSpec::default(),
			modify_fime: TimeSpec::default(),
			change_time: TimeSpec::default(),
		})
	}

	fn chown(&self, owner: usize, group: usize) -> Result<(), Errno> {
		let mut this = self.lock();

		this.owner = owner;
		this.group = group;

		Ok(())
	}

	fn chmod(&self, perm: Permission) -> Result<(), Errno> {
		let mut this = self.lock();

		this.perm = perm;

		Ok(())
	}

	fn mkdir(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn DirInode>, Errno> {
		let mut this = self.lock();

		let ident = Ident::new(name);

		use alloc::collections::btree_map::Entry::*;
		match this.sub_files.entry(ident) {
			Vacant(v) => {
				let current = unsafe { CURRENT.get_mut() };
				let new_dir = TmpDirInode::new_shared(perm, current.get_uid(), current.get_gid());

				v.insert(TmpInode::Dir(new_dir.clone()));

				Ok(new_dir)
			}
			Occupied(_) => Err(Errno::EEXIST),
		}
	}

	fn lookup(&self, name: &[u8]) -> Result<VfsInode, Errno> {
		let this = self.lock();

		this.sub_files
			.get(name)
			.cloned()
			.ok_or(Errno::ENOENT)
			.map(|x| x.into())
	}

	fn rmdir(&self, name: &[u8]) -> Result<(), Errno> {
		use alloc::collections::btree_map::Entry::*;
		let mut this = self.lock();

		let ident = Ident::new(name);
		let entry = match this.sub_files.entry(ident) {
			Vacant(_) => Err(Errno::EEXIST),
			Occupied(o) => Ok(o),
		}?;

		use TmpInode::*;
		match entry.get() {
			Dir(d) => match d.lock().is_empty() {
				true => Ok(()),
				false => Err(Errno::ENOTEMPTY),
			},
			File(_) | SymLink(_) => Err(Errno::ENOTDIR),
		}?;

		entry.remove();

		Ok(())
	}

	fn create(&self, name: &[u8], perm: Permission) -> Result<Arc<dyn FileInode>, Errno> {
		use alloc::collections::btree_map::Entry::*;
		let mut this = self.lock();

		let ident = Ident::new(name);
		match this.sub_files.entry(ident) {
			Vacant(v) => {
				let current = unsafe { CURRENT.get_ref() };
				let new_file = TmpFileInode::new(perm, current.get_uid(), current.get_gid());

				v.insert(TmpInode::File(new_file.clone()));

				Ok(new_file)
			}
			Occupied(_) => Err(Errno::EEXIST),
		}
	}

	fn unlink(&self, name: &[u8]) -> Result<(), Errno> {
		use alloc::collections::btree_map::Entry::*;
		let mut this = self.lock();

		let ident = Ident::new(name);
		let entry = match this.sub_files.entry(ident) {
			Vacant(_) => Err(Errno::ESRCH),
			Occupied(o) => Ok(o),
		}?;

		use TmpInode::*;
		match entry.get() {
			Dir(_) => Err(Errno::EISDIR),
			File(_) | SymLink(_) => Ok(()),
		}?;

		entry.remove();

		Ok(())
	}

	fn symlink(&self, target: &[u8], name: &[u8]) -> Result<Arc<dyn SymLinkInode>, Errno> {
		use alloc::collections::btree_map::Entry::*;
		let mut this = self.lock();

		let ident = Ident::new(name);

		let symlink = Arc::new(TmpSymLink {
			target: Path::new(target),
		});

		match this.sub_files.entry(ident) {
			Vacant(v) => v.insert(TmpInode::SymLink(symlink.clone())),
			Occupied(_) => return Err(Errno::EEXIST),
		};

		Ok(symlink)
	}
}

pub struct TmpSymLink {
	target: Path,
}

impl TmpSymLink {
	pub fn new(target: Path) -> Self {
		Self { target }
	}
}

impl SymLinkInode for TmpSymLink {
	fn target(&self) -> &Path {
		&self.target
	}
}

pub struct TmpDir {
	dirents: Vec<(u8, Vec<u8>)>,
	last: Locked<usize>,
}

impl TmpDir {
	pub fn new(dirents: Vec<(u8, Vec<u8>)>) -> Self {
		Self {
			dirents,
			last: Locked::new(0),
		}
	}
}

impl DirHandle for TmpDir {
	fn getdents(&self, buf: &mut [u8], _io_flags: IOFlag) -> Result<usize, Errno> {
		let mut last = self.last.lock();

		if *last == self.dirents.len() {
			return Ok(0);
		}

		let mut total_size = 0;
		let mut curr_buf = buf;
		for i in *last..self.dirents.len() {
			let (kind, name) = &self.dirents[i];

			let curr_size = next_align(size_of::<KfsDirent>() + name.len() + 1 + 1, 4);

			if curr_buf.len() < curr_size {
				break;
			}

			unsafe {
				let ptr = curr_buf.as_mut_ptr().cast::<KfsDirent>();
				ptr.write(KfsDirent {
					ino: 0,
					private: 0,
					size: curr_size as u16,
					file_type: *kind,
					name: (),
				});

				let name_start = addr_of_mut!((*ptr).name);

				name_start
					.cast::<u8>()
					.copy_from_nonoverlapping(name.as_ptr(), name.len());
				name_start.cast::<u8>().add(name.len()).write(0);
			}

			total_size += curr_size;
			*last += 1;
			(_, curr_buf) = curr_buf.split_at_mut(curr_size);
		}

		if total_size == 0 {
			return Err(Errno::EINVAL);
		}

		Ok(total_size)
	}
}
