use crate::ptr::VirtPageBox;

pub struct KernelModule {
	mem: VirtPageBox,
	init_address: usize,
}

impl KernelModule {
	pub fn new(mem: VirtPageBox, init_offset: usize) -> Self {
		Self {
			mem,
			init_address: init_offset,
		}
	}

	pub fn get_entry_point(&self) -> usize {
		self.init_address
	}
}
