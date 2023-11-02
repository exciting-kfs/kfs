mod chunk;
mod mapped_file;

use alloc::collections::BTreeMap;
use chunk::PageAlignedChunk;

use core::alloc::AllocError;
use core::cmp::min;
use core::mem::size_of;
use core::ptr::NonNull;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;

use crate::config::{MAX_PAGE_PER_ARG, MAX_PAGE_PER_ARGV, TRAMPOLINE_BASE};
use crate::elf::Elf;
use crate::fs::vfs::{RealEntry, VfsHandle, Whence};
use crate::mm::alloc::page::free_pages;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::page::{get_zero_page_phys, index_to_meta, PageFlag, PD};
use crate::mm::{constant::*, util::*};
use crate::pr_warn;
use crate::process::task::Task;
use crate::ptr::PageBox;
use crate::syscall::errno::Errno;
use crate::trace_feature;

use self::mapped_file::MappedFile;

use super::copy::{copy_user_to_user_page, memset_to_user_page};
use super::verify::{verify_ptr, verify_string};
use super::vma::{AreaFlag, UserAddressSpace};

pub struct Memory {
	system_data_base: usize,
	vma: UserAddressSpace,
	file_mapping: BTreeMap<usize, MappedFile>,
	page_dir: PD,
}

extern "C" {
	fn __trampoline_start();
	fn __trampoline_end();
}

impl Memory {
	pub fn from_elf(stack_base: usize, nr_stack_pages: usize, elf: Elf<'_>) -> Result<Self, Errno> {
		let mut memory = Self {
			system_data_base: TRAMPOLINE_BASE,
			vma: UserAddressSpace::new(),
			file_mapping: BTreeMap::new(),
			page_dir: PD::new().map_err(|_| Errno::ENOMEM)?,
		};

		memory.reserve_stack(stack_base, nr_stack_pages)?;

		let len = __trampoline_end as usize - __trampoline_start as usize;
		let trampoline = unsafe { from_raw_parts(__trampoline_start as *const u8, len) };
		memory.push_data(trampoline)?;

		for section in elf.loadable_sections() {
			memory.load_section(section.vaddr, section.data, section.mem_size, section.flags)?;
		}

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

	pub fn brk(&mut self, new_data_end: usize) -> Result<usize, Errno> {
		let begin = next_align(self.system_data_base, PAGE_SIZE);
		let end = next_align(new_data_end, PAGE_SIZE);

		let old = self.system_data_base;

		if end == begin {
			self.system_data_base = new_data_end;
			return Ok(old);
		}

		if end < begin {
			return Err(Errno::ENOMEM);
		}

		self.mmap_private(
			begin,
			(end - begin) / PAGE_SIZE,
			AreaFlag::Readable | AreaFlag::Writable,
		)?;

		self.system_data_base = new_data_end;

		Ok(old)
	}

	pub fn get_data_end(&self) -> usize {
		self.system_data_base
	}

	pub fn mmap_shared(
		&mut self,
		start: usize,
		len: usize,
		file: VfsHandle,
		offset: isize,
		flags: AreaFlag,
	) -> Result<usize, Errno> {
		file.lseek(offset, Whence::Begin)?;

		let file_end_from_offset = file
			.as_entry()
			.and_then(|e| e.downcast_real().ok())
			.and_then(|real| real.stat().ok())
			.and_then(|stat| stat.size.checked_sub(offset))
			.unwrap_or_default();

		let mapping_len = min(len, file_end_from_offset as usize);
		let count = size_to_pages(mapping_len);
		let (start, pages) = self.alloc_memory(start, count, flags)?;

		trace_feature!(
			"mmap_shared",
			"shared start: {:x}, len: {}",
			start,
			mapping_len
		);

		let buf = unsafe { from_raw_parts_mut(start as *mut u8, mapping_len) };
		let mut cursor = 0;
		while cursor < mapping_len {
			let buf = &mut buf[cursor..];
			let x = match file.read(buf) {
				Ok(x) => x,
				Err(e) => {
					self.mmap_cleanup(start, count);
					return Err(e);
				}
			};
			cursor += x;
		}

		self.file_mapping
			.insert(start, MappedFile::new(file, offset, mapping_len));
		pages.into_iter().for_each(|p| p.forget());
		Ok(start)
	}

	fn alloc_memory(
		&mut self,
		start: usize,
		count: usize,
		flags: AreaFlag,
	) -> Result<(usize, Vec<PageBox>), Errno> {
		let mut pages = Vec::new();

		for _ in 0..count {
			pages.push(PageBox::new(Zone::High)?);
		}

		let start = self.alloc_area(start, count, flags)?;

		for (i, page) in (0..count).zip(pages.iter()) {
			if let Err(_) =
				self.page_dir
					.map_user(start + i * PAGE_SIZE, page.as_phys_addr(), flags.into())
			{
				self.mmap_cleanup(start, i);
				return Err(Errno::ENOMEM);
			}
		}

		Ok((start, pages))
	}

	pub fn mmap_private(
		&mut self,
		start: usize,
		pages: usize,
		flags: AreaFlag,
	) -> Result<usize, Errno> {
		let start = self.alloc_area(start, pages, flags)?;

		for i in 0..pages {
			if let Err(_) = self.page_dir.map_user(
				start + i * PAGE_SIZE,
				get_zero_page_phys(),
				PageFlag::Present | PageFlag::User,
			) {
				self.mmap_cleanup(start, i);
				return Err(Errno::ENOMEM);
			}
		}

		Ok(start)
	}

	fn alloc_area(&mut self, start: usize, pages: usize, flags: AreaFlag) -> Result<usize, Errno> {
		if start != 0 {
			self.vma.allocate_fixed_area(start, pages, flags)
		} else {
			Err(AllocError)
		}
		.or_else(|_| self.vma.allocate_area(pages, flags))
		.map_err(|_| Errno::ENOMEM)
	}

	fn mmap_cleanup(&mut self, start: usize, count: usize) {
		self.vma.deallocate_area(start);
		let count = match count.checked_sub(1) {
			Some(x) => x,
			None => return,
		};

		for i in 0..count {
			self.page_dir.unmap_user(start + i * PAGE_SIZE);
		}
	}

	pub fn munmap(&mut self, start: usize, pages: usize) -> Result<(), Errno> {
		let end = start.checked_add(pages * PAGE_SIZE).ok_or(Errno::EINVAL)?;

		let area = self.vma.find_area(start).ok_or(Errno::EINVAL)?;
		if area.start != start || area.end != end {
			return Err(Errno::EINVAL);
		}

		if let Some(mapped_file) = self.file_mapping.remove(&start) {
			mapped_file.sync_with_buf(start as *const u8)?;
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
				let zero_paddr = get_zero_page_phys();

				let paddr = if src_paddr == zero_paddr {
					zero_paddr
				} else if area.flags.contains(AreaFlag::Shared) {
					let index = src_paddr / PAGE_SIZE;

					let mut meta = index_to_meta(index);
					unsafe { meta.as_mut().inc_inuse() };

					trace_feature!("mmap_shared", "clone: inuse: {}", unsafe {
						meta.as_mut().inuse()
					});

					src_paddr
				} else {
					get_copied_page(src_paddr)?
				};

				page_dir.map_user(vaddr, paddr, PageFlag::USER_RDWR)?;
			}
		}
		pr_warn!("SBASE: {}", self.system_data_base);

		Ok(Self {
			system_data_base: self.system_data_base,
			vma,
			page_dir,
			file_mapping: self.file_mapping.clone(),
		})
	}

	pub fn pick_up(&self) {
		self.page_dir.pick_up();
	}

	#[inline]
	pub fn get_pd(&mut self) -> &mut PD {
		&mut self.page_dir
	}

	pub fn get_vma(&self) -> &UserAddressSpace {
		&self.vma
	}

	fn copy_c_argv(
		argv_ptr: usize,
		copy_base: usize,
		task: &Arc<Task>,
	) -> Result<(Vec<u8>, Vec<usize>), Errno> {
		if argv_ptr == 0 {
			return Ok((vec![], vec![0]));
		}

		let mut args: Vec<u8> = Vec::new();
		let mut arg_ptrs: Vec<usize> = Vec::new();
		let mut curr_copy_base = copy_base;

		for i in (0..).step_by(size_of::<usize>()) {
			let argp = verify_ptr::<usize>(argv_ptr + i, task)?;
			if *argp == 0 {
				break;
			}

			let arg = verify_string(*argp, task, MAX_PAGE_PER_ARG * PAGE_SIZE)?;
			if arg.len() + args.len() > MAX_PAGE_PER_ARGV * PAGE_SIZE {
				return Err(Errno::E2BIG);
			}
			args.extend(arg);
			args.push(b'\0');

			arg_ptrs.push(curr_copy_base);

			curr_copy_base += arg.len() + 1;
		}

		arg_ptrs.push(0);

		Ok((args, arg_ptrs))
	}

	pub fn push_string_array(
		&mut self,
		argv_ptr: usize,
		task: &Arc<Task>,
	) -> Result<Vec<usize>, Errno> {
		let copy_base = next_align(self.system_data_base, PAGE_SIZE);

		let (args, arg_ptrs) = Self::copy_c_argv(argv_ptr, copy_base, task)?;

		self.push_data(&args).map_err(|_| Errno::ENOMEM)?;

		Ok(arg_ptrs)
	}

	fn push_data(&mut self, data: &[u8]) -> Result<usize, AllocError> {
		let addr = next_align(self.system_data_base, PAGE_SIZE);

		self.copy_data_at(addr, data)?;

		self.system_data_base = addr + data.len();

		Ok(addr)
	}

	fn load_section(
		&mut self,
		addr: usize,
		data: &[u8],
		len: usize,
		flags: AreaFlag,
	) -> Result<(), Errno> {
		if len == 0 {
			return Ok(());
		}

		let aligned_addr = addr & !(PAGE_SIZE - 1);
		let l_padding = addr % PAGE_SIZE;
		let r_padding = next_align(addr + len, PAGE_SIZE) - (addr + len);

		let needed_pages = (l_padding + len + r_padding) / PAGE_SIZE;

		self.vma
			.allocate_fixed_area(aligned_addr, needed_pages, flags)?;

		for (i, chunk) in PageAlignedChunk::new(addr, data, len).enumerate() {
			let user_page = PageBox::new(Zone::High)?;

			let page = kmap(user_page.as_phys_addr())?;
			unsafe { chunk.write_to_page(page) };
			kunmap(page.as_ptr() as usize);

			self.page_dir.map_user(
				aligned_addr + i * PAGE_SIZE,
				user_page.as_phys_addr(),
				flags.into(),
			)?;

			user_page.forget();
		}

		Ok(())
	}

	fn copy_data_at(&mut self, addr: usize, data: &[u8]) -> Result<(), AllocError> {
		if data.len() == 0 {
			return Ok(());
		}

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
				if let Some(mapped_file) = self.file_mapping.remove(&vaddr) {
					let _ = mapped_file.sync_with_buf(vaddr as *const u8);
				}

				Self::free_page_if_allocated(&self.page_dir, vaddr);
			}
		}
	}
}
