use core::mem::size_of;

use alloc::collections::VecDeque;

use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::{mm::constant::PAGE_SIZE, ptr::PageBox, syscall::errno::Errno};

use super::auxv::AuxEntry;

pub struct UserStack {
	pages: VecDeque<PageBox>,
	next_offset: usize,
}

impl UserStack {
	pub fn new() -> Self {
		Self {
			pages: VecDeque::new(),
			next_offset: PAGE_SIZE,
		}
	}

	pub fn push(&mut self, data: usize) -> Result<(), Errno> {
		if self.next_offset == PAGE_SIZE {
			self.pages
				.push_back(PageBox::new(Zone::High).map_err(|_| Errno::ENOMEM)?);
			self.next_offset -= size_of::<usize>();
		}

		let page = self.pages[self.pages.len() - 1].as_phys_addr();
		let page = kmap(page).map_err(|_| Errno::ENOMEM)?;

		unsafe { *((page.as_ptr().add(self.next_offset)) as *mut usize) = data };

		if self.next_offset == 0 {
			self.next_offset = PAGE_SIZE;
		} else {
			self.next_offset -= size_of::<usize>();
		}

		kunmap(page.as_ptr() as usize);

		Ok(())
	}

	pub fn push_aux_entry(&mut self, aux: AuxEntry) -> Result<(), Errno> {
		let data = aux.serialize();

		for x in data {
			self.push(x)?;
		}

		Ok(())
	}

	pub fn get_stack_pointer(&self, base: usize) -> usize {
		base - self.pages.len() * PAGE_SIZE + self.next_offset + size_of::<usize>()
	}

	pub fn pop_page(&mut self) -> Option<PageBox> {
		self.pages.pop_front()
	}
}
