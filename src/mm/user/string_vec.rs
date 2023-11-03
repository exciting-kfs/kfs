use core::mem::size_of;

use alloc::{sync::Arc, vec::Vec};

use crate::{
	config::{MAX_PAGE_PER_ARG, MAX_PAGE_PER_ARGV},
	mm::constant::PAGE_SIZE,
	process::task::Task,
	syscall::errno::Errno,
};

use super::verify::{verify_ptr, verify_string};

pub struct StringVec {
	pub data: Vec<u8>,
	pub index: Vec<usize>,
}

impl StringVec {
	pub fn new_null() -> Self {
		Self {
			data: Vec::new(),
			index: Vec::new(),
		}
	}

	pub fn new(argv_ptr: usize, task: &Arc<Task>) -> Result<Self, Errno> {
		if argv_ptr == 0 {
			return Ok(Self {
				data: Vec::new(),
				index: Vec::new(),
			});
		}

		let mut data: Vec<u8> = Vec::new();
		let mut index: Vec<usize> = Vec::new();
		let mut curr_idx = 0;

		for i in (0..).step_by(size_of::<usize>()) {
			let argp = verify_ptr::<usize>(argv_ptr + i, task)?;
			if *argp == 0 {
				break;
			}

			let string = verify_string(*argp, task, MAX_PAGE_PER_ARG * PAGE_SIZE)?;
			if string.len() + data.len() > MAX_PAGE_PER_ARGV * PAGE_SIZE {
				return Err(Errno::E2BIG);
			}
			data.extend(string);
			data.push(b'\0');

			index.push(curr_idx);

			curr_idx += string.len() + 1;
		}

		Ok(Self { data, index })
	}

	pub fn len(&self) -> usize {
		self.index.len()
	}
}
