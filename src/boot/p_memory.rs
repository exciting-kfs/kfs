use core::{
	cmp::max,
	mem::{align_of, size_of},
	ops::Range,
};

use multiboot2::{BootInformation, MemoryMapTag};

use crate::{
	mm::{
		constant::VM_OFFSET,
		util::{next_align_64, phys_to_virt},
	},
	sync::singleton::Singleton,
};

use super::Error;

const REQUIRED_MINIMUN_MEMORY_END: u64 = 0x3ffe_0000; // 1024MB
pub static MEM_INFO: Singleton<PMemory> = Singleton::uninit();

#[derive(Clone)]
pub struct PMemory {
	pub linear: Range<u64>,
	pub kernel_end: u64,
}

impl PMemory {
	pub unsafe fn alloc_n<T>(&mut self, n: usize) -> *mut T {
		let begin = next_align_64(self.kernel_end, align_of::<T>() as u64);
		let end = begin + size_of::<T>() as u64 * n as u64;

		let limit = self.linear.end;

		assert!(end <= limit);

		self.kernel_end = end;

		phys_to_virt(begin as usize) as *mut T
	}
}

pub fn init(bi: &BootInformation, kernel_end: usize) -> Result<(), Error> {
	let end_addr = bi.end_address();
	let header_end = end_addr.checked_sub(VM_OFFSET).unwrap_or(end_addr);
	let kernel_end = max(kernel_end as u64, header_end as u64);

	let mmap_tag = bi.memory_map_tag().ok_or_else(|| Error::MissingMemoryMap)?;
	let mem_info = PMemory {
		linear: parse_memory_map(mmap_tag)?,
		kernel_end,
	};

	unsafe { MEM_INFO.write(mem_info) };
	Ok(())
}

fn parse_memory_map(tag: &MemoryMapTag) -> Result<Range<u64>, Error> {
	let linear = tag
		.memory_areas()
		.find(|x| x.start_address() == (1024 * 1024))
		.ok_or_else(|| Error::MissingLinearMemory)?;

	if linear.end_address() >= REQUIRED_MINIMUN_MEMORY_END {
		Ok(linear.start_address()..linear.end_address())
	} else {
		Err(Error::InSufficientMemory)
	}
}
