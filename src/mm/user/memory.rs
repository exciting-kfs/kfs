mod chunk;
mod mapped_file;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use chunk::PageAlignedChunk;

use core::alloc::AllocError;
use core::cmp::min;
use core::mem::size_of;
use core::ptr::NonNull;
use core::slice::{from_raw_parts, from_raw_parts_mut};

use crate::config::{TRAMPOLINE_BASE, USTACK_BASE, USTACK_PAGES};
use crate::elf::{Elf, ProgramHdr};
use crate::fs::path::Path;
use crate::fs::vfs::{Entry, VfsHandle, Whence};
use crate::mm::alloc::page::free_pages;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::page::{get_zero_page_phys, index_to_meta, PageFlag, PD};
use crate::mm::{constant::*, util::*};
use crate::process::task::CURRENT;
use crate::ptr::PageBox;
use crate::syscall::errno::Errno;
use crate::syscall::exec::read_user_binary;
use crate::trace_feature;

use self::mapped_file::MappedFile;

use super::auxv::{AuxEntry, AuxEntryType};
use super::copy::{copy_user_to_user_page, memset_to_user_page};
use super::stack::UserStack;
use super::string_vec::StringVec;
use super::vma::{AreaFlag, UserAddressSpace};

pub struct Memory {
	stack_pointer: usize,
	system_data_base: usize,
	pub entry_point: usize,
	vma: UserAddressSpace,
	file_mapping: BTreeMap<usize, MappedFile>,
	page_dir: PD,
}

extern "C" {
	fn __trampoline_start();
	fn __trampoline_end();
}

impl Memory {
	fn load_interp(&mut self, interp: Path) -> Result<usize, Errno> {
		let current = unsafe { CURRENT.get_ref() };
		let raw_bin = read_user_binary(interp, current)?;
		let elf = Elf::new(raw_bin.as_slice()).map_err(|_| Errno::ENOEXEC)?;

		let interp_base = match elf.is_position_independent() {
			true => 0x400000,
			false => 0,
		};

		self.load_sections(interp_base, &elf)?;

		self.entry_point = interp_base + elf.get_entry_point();

		Ok(interp_base)
	}

	pub fn from_elf(elf: Elf<'_>, argv: StringVec, envp: StringVec) -> Result<Self, Errno> {
		let mut memory = Self {
			entry_point: 0,
			stack_pointer: USTACK_BASE,
			system_data_base: TRAMPOLINE_BASE,
			vma: UserAddressSpace::new(),
			file_mapping: BTreeMap::new(),
			page_dir: PD::new().map_err(|_| Errno::ENOMEM)?,
		};

		let len = __trampoline_end as usize - __trampoline_start as usize;
		let trampoline = unsafe { from_raw_parts(__trampoline_start as *const u8, len) };
		memory.push_data(trampoline)?;

		let executable_base = match elf.is_position_independent() {
			true => 0x0804_8000,
			false => 0,
		};

		memory.entry_point = executable_base + elf.get_entry_point();

		memory.load_sections(executable_base, &elf)?;

		let mut stack = UserStack::new();

		stack.push_aux_entry(AuxEntry::new_null())?;

		if let Some(interp) = elf.get_interpreter() {
			let interp_base = memory.load_interp(interp)?;
			stack.push_aux_entry(AuxEntry::new(AuxEntryType::Base, interp_base))?;
			stack.push_aux_entry(AuxEntry::new(
				AuxEntryType::Phdr,
				executable_base + elf.program_hdrs[0].p_vaddr,
			))?;
			stack.push_aux_entry(AuxEntry::new(AuxEntryType::Phent, size_of::<ProgramHdr>()))?;
			stack.push_aux_entry(AuxEntry::new(AuxEntryType::Phnum, elf.program_hdrs.len()))?;
			stack.push_aux_entry(AuxEntry::new(
				AuxEntryType::Entry,
				executable_base + elf.get_entry_point(),
			))?;
		}

		stack.push_aux_entry(AuxEntry::new(AuxEntryType::Pagesz, PAGE_SIZE))?;

		memory.push_string_array(envp, &mut stack)?;

		let argc = argv.len();
		memory.push_string_array(argv, &mut stack)?;

		stack.push(argc)?;

		memory.reserve_stack(stack)?;

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
			.and_then(|ent| ent.stat().ok())
			.and_then(|stat| {
				offset
					.try_into()
					.ok()
					.and_then(|offset| stat.size.checked_sub(offset))
			})
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

		Ok(Self {
			entry_point: self.entry_point,
			stack_pointer: self.stack_pointer,
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

	fn push_string_array(&mut self, strv: StringVec, stack: &mut UserStack) -> Result<(), Errno> {
		let copy_base = next_align(self.system_data_base, PAGE_SIZE);

		let StringVec { data, index } = strv;

		self.push_data(&data).map_err(|_| Errno::ENOMEM)?;

		stack.push(0)?;
		for elem in index {
			stack.push(elem + copy_base)?;
		}

		Ok(())
	}

	fn push_data(&mut self, data: &[u8]) -> Result<usize, AllocError> {
		let addr = next_align(self.system_data_base, PAGE_SIZE);

		self.copy_data_at(addr, data)?;

		self.system_data_base = addr + data.len();

		Ok(addr)
	}

	fn load_sections(&mut self, base_addr: usize, elf: &Elf<'_>) -> Result<(), Errno> {
		for section in elf.loadable_sections() {
			self.load_section(
				base_addr + section.vaddr,
				section.data,
				section.mem_size,
				section.flags,
			)?;
		}

		Ok(())
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

	fn reserve_stack(&mut self, mut stack: UserStack) -> Result<(), AllocError> {
		self.stack_pointer = stack.get_stack_pointer(USTACK_BASE);

		let stack_top = USTACK_BASE - (USTACK_PAGES * PAGE_SIZE);

		self.vma.allocate_fixed_area(
			stack_top,
			USTACK_PAGES,
			AreaFlag::Readable | AreaFlag::Writable,
		)?;

		for user_vaddr in (1..=USTACK_PAGES).map(|x| USTACK_BASE - x * PAGE_SIZE) {
			let user_page = match stack.pop_page() {
				Some(page) => page,
				None => {
					let page = PageBox::new(Zone::High)?;

					unsafe { memset_to_user_page(page.as_phys_addr(), 0)? };

					page
				}
			};

			self.page_dir
				.map_user(user_vaddr, user_page.as_phys_addr(), PageFlag::USER_RDWR)?;

			user_page.forget();
		}

		Ok(())
	}

	pub fn get_stack_pointer(&self) -> usize {
		self.stack_pointer
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
