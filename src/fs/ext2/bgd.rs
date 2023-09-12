use core::fmt::{Debug, Display};

use alloc::{boxed::Box, vec::Vec};

use crate::write_field;

/// Block Group Descripotr
#[repr(C)]
pub struct BGD {
	pub block_bitmap: u32,
	pub inode_bitmap: u32,
	pub inode_table: u32,
	pub free_blocks_count: u16,
	pub free_inodes_count: u16,
	pub used_dirs_count: u16,
	_pad: u16,
	_reserved: [u32; 3],
}

impl Debug for BGD {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		Display::fmt(&self, f)
	}
}

impl Display for BGD {
	fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
		write!(f, "\n")?;
		write_field!(f, self, block_bitmap)?;
		write_field!(f, self, inode_bitmap)?;
		write_field!(f, self, inode_table)?;
		write_field!(f, self, free_blocks_count)?;
		write_field!(f, self, free_inodes_count)?;
		write_field!(f, self, used_dirs_count)?;

		Ok(())
	}
}

pub struct BGDT(Vec<Box<[BGD]>>);

impl BGDT {
	pub fn new(v: Vec<Box<[BGD]>>) -> Option<Self> {
		if v.len() == 0 {
			None
		} else {
			Some(Self(v))
		}
	}

	pub fn get_bgd(&self, bgid: usize) -> Option<&BGD> {
		let chunk_count = self.0.len();
		let step_size = self.0[0].len();
		let step = bgid / step_size;

		if step >= chunk_count {
			return None;
		}

		let index = bgid % step_size;
		Some(&self.0[step][index])
	}

	pub fn find_bgd<F>(&mut self, condition: F) -> Option<(usize, &mut BGD)>
	where
		F: Fn(&mut BGD) -> bool,
	{
		let mut bgid = 0;
		for chunk in self.0.iter_mut() {
			for bgd in chunk.iter_mut() {
				if condition(bgd) {
					return Some((bgid, bgd));
				}
				bgid += 1;
			}
		}
		None
	}
}
