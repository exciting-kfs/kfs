use core::alloc::AllocError;
use core::ptr::NonNull;
use core::slice::from_raw_parts;

use crate::config::TRAMPOLINE_BASE;
use crate::mm::alloc::page::free_pages;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::page::{get_zero_page_phys, PageFlag, PD};
use crate::mm::{constant::*, util::*};
use crate::ptr::PageBox;
use crate::syscall::errno::Errno;

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
		let end = match start.checked_add(bytes) {
			Some(x) => x,
			None => return false,
		};

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

	pub fn mmap_private(
		&mut self,
		start: usize,
		pages: usize,
		flags: AreaFlag,
	) -> Result<usize, Errno> {
		fn cleanup(memory: &mut Memory, start: usize, count: usize) {
			memory.vma.deallocate_area(start);
			let count = match count.checked_sub(1) {
				Some(x) => x,
				None => return,
			};

			for i in 0..count {
				memory.page_dir.unmap_user(start + i * PAGE_SIZE);
			}
		}

		let start = if start != 0 {
			self.vma.allocate_fixed_area(start, pages, flags)
		} else {
			Err(AllocError)
		}
		.or_else(|_| self.vma.allocate_area(pages, flags))
		.map_err(|_| Errno::ENOMEM)?;

		for i in 0..pages {
			if let Err(_) = self.page_dir.map_user(
				start + i * PAGE_SIZE,
				get_zero_page_phys(),
				PageFlag::Present | PageFlag::User,
			) {
				cleanup(self, start, i);
				return Err(Errno::ENOMEM);
			}
		}

		Ok(start)
	}

	pub fn munmap_private(&mut self, start: usize, pages: usize) -> Result<(), Errno> {
		let end = start.checked_add(pages * PAGE_SIZE).ok_or(Errno::EINVAL)?;

		let area = self.vma.find_area(start).ok_or(Errno::EINVAL)?;
		if area.start != start || area.end != end {
			return Err(Errno::EINVAL);
		}

		self.vma.deallocate_area(start).unwrap();

		for vaddr in (0..pages).map(|x| start + x * PAGE_SIZE) {
			Self::free_page_if_allocated(self.get_pd(), vaddr);
			self.page_dir.unmap_user(vaddr);
		}

		Ok(())
	}

	pub fn clone(&self) -> Result<Self, AllocError> {
		fn get_copied_page(src_paddr: usize) -> Result<usize, AllocError> {
			let page = PageBox::new(Zone::High)?;

			unsafe { copy_user_to_user_page(src_paddr, page.as_phys_addr())? };

			let paddr = page.as_phys_addr();

			page.forget();

			Ok(paddr)
		}

		let vma = self.vma.clone();
		let mut page_dir = PD::new()?;

		for area in vma.get_areas() {
			for vaddr in (area.start..area.end).step_by(PAGE_SIZE) {
				let src_paddr = self.page_dir.lookup(vaddr).unwrap();

				let paddr = if src_paddr != get_zero_page_phys() {
					get_copied_page(src_paddr)?
				} else {
					get_zero_page_phys()
				};

				page_dir.map_user(vaddr, paddr, PageFlag::USER_RDWR)?;
			}
		}

		Ok(Self { vma, page_dir })
	}

	pub fn pick_up(&self) {
		self.page_dir.pick_up();
	}

	pub fn get_pd(&mut self) -> &mut PD {
		&mut self.page_dir
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

	fn free_page_if_allocated(pd: &PD, vaddr: usize) -> Option<()> {
		let paddr = pd.lookup(vaddr)?;

		if get_zero_page_phys() != paddr {
			free_pages(unsafe { NonNull::new_unchecked(phys_to_virt(paddr) as *mut u8) })
		}

		Some(())
	}
}

impl Drop for Memory {
	fn drop(&mut self) {
		for area in self.vma.get_areas() {
			for vaddr in area.iter_pages() {
				Self::free_page_if_allocated(&self.page_dir, vaddr);
			}
		}
	}
}
