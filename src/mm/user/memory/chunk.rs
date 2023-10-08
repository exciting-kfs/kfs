use core::ptr::NonNull;

use crate::mm::constant::PAGE_SIZE;

pub struct PageAlignedChunk<'a> {
	next_offset: usize,
	remain_bytes: usize,
	data: &'a [u8],
}

impl<'a> PageAlignedChunk<'a> {
	pub fn new(start_addr: usize, data: &'a [u8], total_bytes: usize) -> Self {
		let next_offset = start_addr % PAGE_SIZE;

		Self {
			next_offset,
			remain_bytes: total_bytes,
			data,
		}
	}
}

impl<'a> Iterator for PageAlignedChunk<'a> {
	type Item = ZeroExtendedChunk<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.remain_bytes == 0 {
			return None;
		}

		let data_len = self.data.len().min(PAGE_SIZE - self.next_offset);

		let (curr, next) = self.data.split_at(data_len);
		let chunk = ZeroExtendedChunk::new(curr, self.next_offset);

		self.remain_bytes = self
			.remain_bytes
			.checked_sub(PAGE_SIZE - self.next_offset)
			.unwrap_or_default();

		self.next_offset = 0;
		self.data = next;

		Some(chunk)
	}
}

pub struct ZeroExtendedChunk<'a> {
	offset: usize,
	data: &'a [u8],
}

impl<'a> ZeroExtendedChunk<'a> {
	pub fn new(data: &'a [u8], offset: usize) -> Self {
		debug_assert!(data.len() + offset <= PAGE_SIZE);

		Self { offset, data }
	}

	pub unsafe fn write_to_page(&self, page: NonNull<u8>) {
		let left_zeros_len = self.offset;
		let data_len = self.data.len();
		let right_zeros_len = PAGE_SIZE - (left_zeros_len + data_len);

		let mut page = page.as_ptr();

		page.write_bytes(0, left_zeros_len);
		page = page.add(left_zeros_len);

		page.copy_from_nonoverlapping(self.data.as_ptr(), data_len);
		page = page.add(data_len);

		page.write_bytes(0, right_zeros_len);
	}
}

mod test {
	use core::ptr::NonNull;

	use crate::util::lcg::LCG;

	use super::*;
	use alloc::{boxed::Box, vec::Vec};
	use kfs_macro::ktest;

	#[ktest(memory)]
	pub fn page_chunk_basic() {
		let mut buffer = Box::new([42; PAGE_SIZE * 2]);

		for (i, chunk) in PageAlignedChunk::new(0, &[1, 2, 3, 4], 4).enumerate() {
			assert!(i < 1);
			unsafe {
				let page = NonNull::new_unchecked(buffer.as_mut_ptr().add(i * PAGE_SIZE));

				chunk.write_to_page(page);
			}
		}

		assert_eq!(buffer[0], 1);
		assert_eq!(buffer[1], 2);
		assert_eq!(buffer[2], 3);
		assert_eq!(buffer[3], 4);

		for ch in &buffer[4..PAGE_SIZE] {
			assert_eq!(*ch, 0);
		}

		for ch in &buffer[PAGE_SIZE..] {
			assert_eq!(*ch, 42);
		}
	}

	#[ktest(memory)]
	pub fn page_chunk_misalign_onepage() {
		let mut buffer = Box::new([42; PAGE_SIZE * 2]);

		for (i, chunk) in PageAlignedChunk::new(1, &[1, 2, 3, 4], 4).enumerate() {
			assert!(i < 1);
			unsafe {
				let page = NonNull::new_unchecked(buffer.as_mut_ptr().add(i * PAGE_SIZE));

				chunk.write_to_page(page);
			}
		}

		assert_eq!(buffer[0], 0);
		assert_eq!(buffer[1], 1);
		assert_eq!(buffer[2], 2);
		assert_eq!(buffer[3], 3);
		assert_eq!(buffer[4], 4);

		for ch in &buffer[5..PAGE_SIZE] {
			assert_eq!(*ch, 0);
		}

		for ch in &buffer[PAGE_SIZE..] {
			assert_eq!(*ch, 42);
		}
	}

	#[ktest(memory)]
	pub fn page_chunk_misalign_multipage() {
		let mut buffer = Box::new([42; PAGE_SIZE * 3]);
		let mut data = Vec::new();

		let mut rng = LCG::new(42);

		for _ in 0..5000 {
			data.push(rng.rand() as u8);
		}

		for (i, chunk) in PageAlignedChunk::new(42, &data, data.len()).enumerate() {
			assert!(i < 2);
			unsafe {
				let page = NonNull::new_unchecked(buffer.as_mut_ptr().add(i * PAGE_SIZE));

				chunk.write_to_page(page);
			}
		}

		for ch in &buffer[..42] {
			assert_eq!(*ch, 0);
		}

		for (a, b) in (data.iter()).zip(&buffer[42..42 + data.len()]) {
			assert_eq!(*a, *b);
		}

		for ch in &buffer[42 + data.len()..2 * PAGE_SIZE] {
			assert_eq!(*ch, 0);
		}

		for ch in &buffer[2 * PAGE_SIZE..] {
			assert_eq!(*ch, 42);
		}
	}

	#[ktest(memory)]
	pub fn page_chunk_extend() {
		let mut buffer = Box::new([42; PAGE_SIZE * 2]);
		let mut data = Vec::new();

		let mut rng = LCG::new(42);

		for _ in 0..5000 {
			data.push(rng.rand() as u8);
		}

		for (i, chunk) in PageAlignedChunk::new(0, &[1, 2, 3, 4], 6).enumerate() {
			assert!(i < 1);
			unsafe {
				let page = NonNull::new_unchecked(buffer.as_mut_ptr().add(i * PAGE_SIZE));

				chunk.write_to_page(page);
			}
		}

		assert_eq!(buffer[0], 1);
		assert_eq!(buffer[1], 2);
		assert_eq!(buffer[2], 3);
		assert_eq!(buffer[3], 4);

		for ch in &buffer[4..PAGE_SIZE] {
			assert_eq!(*ch, 0);
		}

		for ch in &buffer[PAGE_SIZE..] {
			assert_eq!(*ch, 42);
		}
	}

	#[ktest(memory)]
	pub fn page_chunk_extend_misaligned() {
		let mut buffer = Box::new([42; PAGE_SIZE * 2]);
		let mut data = Vec::new();

		let mut rng = LCG::new(42);

		for _ in 0..5000 {
			data.push(rng.rand() as u8);
		}

		for (i, chunk) in PageAlignedChunk::new(1, &[1, 2, 3, 4], 6).enumerate() {
			assert!(i < 1);
			unsafe {
				let page = NonNull::new_unchecked(buffer.as_mut_ptr().add(i * PAGE_SIZE));

				chunk.write_to_page(page);
			}
		}

		assert_eq!(buffer[0], 0);
		assert_eq!(buffer[1], 1);
		assert_eq!(buffer[2], 2);
		assert_eq!(buffer[3], 3);
		assert_eq!(buffer[4], 4);

		for ch in &buffer[5..PAGE_SIZE] {
			assert_eq!(*ch, 0);
		}

		for ch in &buffer[PAGE_SIZE..] {
			assert_eq!(*ch, 42);
		}
	}
}
