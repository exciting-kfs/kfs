use core::alloc::AllocError;
use core::ptr::NonNull;
use core::slice::from_raw_parts;

use crate::config::TRAMPOLINE_BASE;
use crate::mm::alloc::page::free_pages;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::page::{PageFlag, PD};
use crate::mm::{constant::*, util::*};
use crate::ptr::PageBox;

use super::copy::{copy_user_to_user_page, memset_to_user_page};
use super::vma::{AreaFlag, UserAddressSpace};

pub struct Memory {
	vma: UserAddressSpace,
	page_dir: PD,
}

extern "C" {
	fn __trampoline_start();
	fn __trampoline_end();
}

impl Memory {
	pub fn new(
		stack_base: usize,
		nr_stack_pages: usize,
		code_base: usize,
		code: &[u8],
	) -> Result<Self, AllocError> {
		let mut memory = Self {
			vma: UserAddressSpace::new(),
			page_dir: PD::new()?,
		};

		memory.reserve_stack(stack_base, nr_stack_pages)?;
		memory.copy_data_at(code_base, code)?;

		let len = __trampoline_end as usize - __trampoline_start as usize;
		let trampoline = unsafe { from_raw_parts(__trampoline_start as *const u8, len) };
		memory.copy_data_at(TRAMPOLINE_BASE, trampoline)?;

		Ok(memory)
	}

	pub fn query_flags_range(&self, start: usize, bytes: usize, flags: AreaFlag) -> bool {
		let end = start + bytes;
		let mut curr = start;

		while curr < end {
			if let Some(a) = self.vma.find_area(curr) {
				if a.flags.contains(flags) {
					curr = a.end;
				} else {
					return false;
				}
			} else {
				return false;
			}
		}
		return true;
	}

	pub fn clone(&self) -> Result<Self, AllocError> {
		let vma = self.vma.clone();
		let mut page_dir = PD::new()?;

		for area in vma.get_areas() {
			for vaddr in (area.start..area.end).step_by(PAGE_SIZE) {
				let src_paddr = self.page_dir.lookup(vaddr).unwrap();
				let dst_page = PageBox::new(Zone::High)?;

				unsafe { copy_user_to_user_page(src_paddr, dst_page.as_phys_addr())? };

				page_dir.map_user(vaddr, dst_page.as_phys_addr(), PageFlag::USER_RDWR)?;

				dst_page.forget();
			}
		}

		Ok(Self { vma, page_dir })
	}

	pub fn pick_up(&self) {
		self.page_dir.pick_up();
	}

	pub fn get_pd(&self) -> &PD {
		&self.page_dir
	}

	pub fn get_vma(&self) -> &UserAddressSpace {
		&self.vma
	}

	fn copy_data_at(&mut self, addr: usize, data: &[u8]) -> Result<(), AllocError> {
		self.vma.allocate_fixed_area(
			addr,
			(data.len() / PAGE_SIZE) + (data.len() % PAGE_SIZE != 0) as usize,
			AreaFlag::Readable | AreaFlag::Writable,
		)?;

		for (i, chunk) in data.chunks(PAGE_SIZE).enumerate() {
			let user_page = PageBox::new(Zone::High)?;

			let temp_ptr = kmap(user_page.as_phys_addr())?;
			unsafe {
				temp_ptr
					.as_ptr()
					.copy_from_nonoverlapping(chunk.as_ptr(), chunk.len())
			};
			if chunk.len() != PAGE_SIZE {
				unsafe {
					temp_ptr
						.as_ptr()
						.add(chunk.len())
						.write_bytes(0, PAGE_SIZE - chunk.len())
				};
			}
			kunmap(temp_ptr.as_ptr() as usize);

			self.page_dir.map_user(
				addr + i * PAGE_SIZE,
				user_page.as_phys_addr(),
				PageFlag::USER_RDWR,
			)?;

			user_page.forget();
		}

		Ok(())
	}

	fn reserve_stack(&mut self, stack_base: usize, nr_pages: usize) -> Result<(), AllocError> {
		let stack_top = stack_base - (nr_pages * PAGE_SIZE);

		self.vma.allocate_fixed_area(
			stack_top,
			nr_pages,
			AreaFlag::Readable | AreaFlag::Writable,
		)?;

		for user_vaddr in (0..nr_pages).map(|x| stack_top + x * PAGE_SIZE) {
			let user_page = PageBox::new(Zone::High)?;

			unsafe { memset_to_user_page(user_page.as_phys_addr(), 0)? };

			self.page_dir
				.map_user(user_vaddr, user_page.as_phys_addr(), PageFlag::USER_RDWR)?;

			user_page.forget();
		}

		Ok(())
	}
}

impl Drop for Memory {
	fn drop(&mut self) {
		for area in self.vma.get_areas() {
			for vaddr in area.iter_pages() {
				if let Some(paddr) = self.page_dir.lookup(vaddr) {
					free_pages(unsafe { NonNull::new_unchecked(phys_to_virt(paddr) as *mut u8) })
				}
			}
		}
	}
}
