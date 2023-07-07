use core::{
	cmp::max,
	mem::{align_of, size_of},
	ptr::NonNull,
	slice::from_raw_parts,
};

use multiboot2::{BootInformation, MemoryMapTag};

use crate::mm::{constant::*, util::*};

use super::Error;

// safety: this will be written only once at early-boot stage.
//  after that this is read only.
pub static mut MEM_INFO: PMemory = PMemory {
	normal_start_pfn: 0,
	high_start_pfn: 0,
	end_pfn: 0,
};

#[derive(Debug, Clone)]
pub struct PMemory {
	pub normal_start_pfn: usize,
	pub high_start_pfn: usize,
	pub end_pfn: usize,
}

pub struct BootAlloc {
	offset: usize,
}

impl BootAlloc {
	pub(super) fn new() -> Self {
		Self { offset: 0 }
	}

	pub fn alloc_n<T>(&mut self, n: usize) -> NonNull<[T]> {
		// MAX supported alignment is `PAGE_SIZE`
		let align = next_align(self.offset, align_of::<T>()) - self.offset;
		let size = size_of::<T>() * n;

		let mem = unsafe { &mut MEM_INFO };

		let alloc_begin = pfn_to_addr(mem.normal_start_pfn) + self.offset + align;

		let mut new_offset = self.offset + align + size;
		let nr_extra_pages = new_offset / PAGE_SIZE;
		new_offset %= PAGE_SIZE;

		if mem.normal_start_pfn + nr_extra_pages > mem.high_start_pfn {
			panic!("bootalloc: out of memory");
		}

		mem.normal_start_pfn += nr_extra_pages;
		self.offset = new_offset;

		// safety: virtual address starts from `VM_OFFSET` which not contains null.
		unsafe { NonNull::from(from_raw_parts(phys_to_virt(alloc_begin) as *mut T, n)) }
	}

	pub fn deinit(self) {
		if self.offset != 0 {
			// safety: this is early boot stage. no synchonization needed.
			let mem = unsafe { &mut MEM_INFO };
			if mem.normal_start_pfn == mem.end_pfn {
				panic!("bootalloc: out of memory");
			}
			mem.normal_start_pfn += 1;
		}
	}
}

pub fn init(bi: &BootInformation, kernel_end: usize) -> Result<(), Error> {
	let header_end = bi.end_address();
	let normal_start_pfn = addr_to_pfn(virt_to_phys(max(kernel_end, header_end)));

	let mmap_tag = bi.memory_map_tag().ok_or_else(|| Error::MissingMemoryMap)?;
	let end_pfn = search_memory_end(mmap_tag)?;

	let high_start_pfn = end_pfn.min(addr_to_pfn(virt_to_phys(VMALLOC_OFFSET)));

	unsafe {
		MEM_INFO = PMemory {
			normal_start_pfn,
			high_start_pfn,
			end_pfn,
		}
	};

	Ok(())
}

fn search_memory_end(tag: &MemoryMapTag) -> Result<usize, Error> {
	let linear = tag
		.memory_areas()
		.find(|x| x.start_address() == (1024 * 1024))
		.ok_or_else(|| Error::MissingLinearMemory)?;

	Ok((linear.end_address() / PAGE_SIZE as u64) as usize)
}
