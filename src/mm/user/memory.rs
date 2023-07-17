use core::alloc::AllocError;

use crate::mm::alloc::page::alloc_pages;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::page::{PageFlag, PD};
use crate::mm::{constant::*, util::*};

use super::copy::{copy_user_to_user_page, memset_to_user_page};
use super::vma::{AreaFlag, UserAddressSpace};

pub struct Memory {
	vma: UserAddressSpace,
	page_dir: PD,
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
		memory.copy_data_at(code_base, code).expect("CDATA");

		Ok(memory)
	}

	pub fn clone(&self) -> Result<Self, AllocError> {
		let vma = self.vma.clone();
		let mut page_dir = PD::new()?;

		for area in vma.get_areas() {
			for vaddr in (area.start..area.end).step_by(PAGE_SIZE) {
				let src_paddr = self.page_dir.lookup(vaddr).unwrap();
				let dst_paddr = virt_to_phys(
					alloc_pages(0, Zone::High).unwrap().as_ptr().cast::<u8>() as usize,
				);

				unsafe { copy_user_to_user_page(src_paddr, dst_paddr)? };

				page_dir.map_user(vaddr, dst_paddr, PageFlag::USER_RDWR)?;
			}
		}

		Ok(Self { vma, page_dir })
	}

	pub fn pick_up(&self) {
		self.page_dir.pick_up();
	}

	fn copy_data_at(&mut self, addr: usize, data: &[u8]) -> Result<(), AllocError> {
		self.vma.allocate_fixed_area(
			addr,
			(data.len() / PAGE_SIZE) + 1,
			AreaFlag::Readable | AreaFlag::Writable,
		)?;

		for (i, chunk) in data.chunks(PAGE_SIZE).enumerate() {
			let user_page = alloc_pages(0, Zone::High)?;
			let user_paddr = virt_to_phys(user_page.as_ptr().cast::<u8>() as usize);

			let temp_ptr = kmap(user_paddr)?;
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

			self.page_dir
				.map_user(addr + i * PAGE_SIZE, user_paddr, PageFlag::USER_RDWR)?;
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
			let user_page = alloc_pages(0, Zone::High)?;
			let user_paddr = virt_to_phys(user_page.as_ptr().cast::<u8>() as usize);

			unsafe { memset_to_user_page(user_paddr, 0)? };

			self.page_dir
				.map_user(user_vaddr, user_paddr, PageFlag::USER_RDWR)?;
		}

		Ok(())
	}
}
