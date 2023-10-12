use core::mem::{size_of, transmute};

use crate::{
	fs::{ext2::inode::inum::Inum, vfs::FileType},
	mm::util::next_align,
};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct Record {
	pub ino: u32,
	rec_len: u16,
	name_len: u8,
	pub file_type: FileType,
	pub name: (),
}

impl Record {
	pub const ALIGN: usize = 4;
	pub fn remain_space(&self) -> usize {
		let total = self.rec_len as usize;

		total - self.len()
	}

	pub fn is_allocatable(&self, name: &[u8]) -> bool {
		let remain = self.remain_space();
		remain >= Self::capacity_need(name) as usize
	}

	#[inline]
	pub fn name_len(&self) -> usize {
		self.name_len as usize
	}

	#[inline]
	pub fn capacity(&self) -> usize {
		self.rec_len as usize
	}

	pub fn capacity_add(&mut self, add: usize) {
		self.rec_len += add as u16;
	}

	pub fn capacity_sub(&mut self, sub: usize) {
		self.rec_len -= sub as u16;
	}

	#[inline]
	pub fn len(&self) -> usize {
		next_align(size_of::<Record>() + self.name_len as usize, Self::ALIGN)
	}

	pub fn capacity_need(name: &[u8]) -> u16 {
		next_align(size_of::<Record>() + name.len(), Self::ALIGN) as u16
	}

	pub fn new_dir(inum: Inum, name_len: u8, capacity: u16) -> Self {
		Self {
			ino: inum.ino() as u32,
			rec_len: capacity,
			name_len,
			file_type: FileType::Directory,
			name: (),
		}
	}

	pub fn new_dir_with_name(
		inum: Inum,
		capacity: u16,
		name_len: u8,
		name: &[u8; 4],
	) -> impl Iterator<Item = u8> {
		let rec = Self {
			ino: inum.ino() as u32,
			rec_len: capacity,
			name_len,
			file_type: FileType::Directory,
			name: (),
		};

		let rec: [u8; size_of::<Record>()] = unsafe { transmute(rec) };
		rec.into_iter().chain(name.clone())
	}

	pub fn new_file(inum: Inum, name_len: u8, rec_len: u16) -> Self {
		Self {
			ino: inum.ino() as u32,
			rec_len,
			name_len,
			file_type: FileType::Regular,
			name: (),
		}
	}

	pub fn new_symlink(inum: Inum, name_len: u8, rec_len: u16) -> Self {
		Self {
			ino: inum.ino() as u32,
			rec_len,
			name_len,
			file_type: FileType::SymLink,
			name: (),
		}
	}
}
