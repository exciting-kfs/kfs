use crate::mm::{
	constant::{PAGE_SIZE, PT_COVER_SIZE},
	util::{phys_to_virt, virt_to_phys},
	GFP, PAGE_ALLOC,
};
use bitflags::bitflags;
use core::array;
use core::ops::{Deref, DerefMut};

use super::util::invalidate_all_tlb;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
	pub struct PageFlag: u32 {
		const Present = 1;
		const Write = 2;
		const User = 4;
		const PWT = 8;
		const PCD = 16;
		const Accessed = 32;
		const Dirty = 64;
		const Global = 256;
	}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PDE {
	data: PageFlag,
}

impl PDE {
	const PSE: u32 = 128;
	const ADDR_MASK_4M: u32 = 0b11111111_11000000_00000000_00000000;
	const ADDR_MASK: u32 = 0b11111111_11111111_11110000_00000000;

	pub fn new_4m(addr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain((addr as u32 & Self::ADDR_MASK_4M) | Self::PSE)
				| flags,
		}
	}

	pub fn new(addr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
		}
	}

	pub fn is_4m(&self) -> bool {
		(self.data.bits() & Self::PSE) != 0
	}

	pub fn addr(&self) -> usize {
		(self.data.bits() & Self::ADDR_MASK) as usize
	}

	pub fn flag(&self) -> PageFlag {
		PageFlag::from_bits_truncate(self.data.bits())
	}
}

impl AsMut<PageFlag> for PDE {
	fn as_mut(&mut self) -> &mut PageFlag {
		&mut self.data
	}
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct PTE {
	data: PageFlag,
}

impl PTE {
	const ADDR_MASK: u32 = 0b11111111_11111111_11110000_00000000;

	pub fn new(addr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain(addr as u32 & Self::ADDR_MASK) | flags,
		}
	}

	pub fn set_flag(&mut self, flag: PageFlag) {
		self.data = flag
	}

	pub fn addr(&self) -> usize {
		(self.data.bits() & Self::ADDR_MASK) as usize
	}

	pub fn flag(&self) -> PageFlag {
		PageFlag::from_bits_truncate(self.data.bits())
	}
}

impl AsMut<PageFlag> for PTE {
	fn as_mut(&mut self) -> &mut PageFlag {
		&mut self.data
	}
}

#[repr(C, align(4096))]
pub struct PT {
	entries: [PTE; 1024],
}

impl PT {
	pub fn new_from_4m(pde_4m: PDE) -> Result<&'static mut Self, ()> {
		let addr = pde_4m.addr();
		let flag = pde_4m.flag();
		unsafe {
			let page_table = PAGE_ALLOC
				.lock()
				.alloc_page(0, GFP::Normal)?
				.cast::<PT>()
				.as_ptr();

			page_table.write(Self {
				entries: array::from_fn(|i| PTE::new(addr + PAGE_SIZE * i, flag)),
			});

			return Ok(page_table.as_mut().unwrap());
		}
	}
}

impl Deref for PT {
	type Target = [PTE; 1024];

	fn deref(&self) -> &Self::Target {
		&self.entries
	}
}

impl DerefMut for PT {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.entries
	}
}

#[repr(C, align(4096))]
pub struct PD {
	entries: [PDE; 1024],
}

impl PD {
	pub fn map_page(&mut self, vaddr: usize, paddr: usize, flags: PageFlag) -> Result<(), ()> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self[pd_idx];

		let pt = if pde.is_4m() {
			let pt = PT::new_from_4m(*pde)?;

			*pde = PDE::new(
				virt_to_phys(pt as *mut PT as usize),
				pde.flag() | PageFlag::Present,
			);

			pt
		} else {
			unsafe { (phys_to_virt(pde.addr()) as *mut PT).as_mut().unwrap() }
		};

		pt[pt_idx] = PTE::new(paddr, flags);

		// TODO: invalidate just single page
		invalidate_all_tlb();

		Ok(())
	}

	pub fn unmap_page(&mut self, vaddr: usize) -> Result<(), ()> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self[pd_idx];

		if !pde.is_4m() {
			let pt = unsafe { (phys_to_virt(pde.addr()) as *mut PT).as_mut().unwrap() };

			let mut new_flag = pt[pt_idx].flag();
			new_flag.remove(PageFlag::Present);

			pt[pt_idx].set_flag(new_flag);

			Ok(())
		} else {
			Err(())
		}
	}

	pub fn lookup(&self, vaddr: usize) -> Option<usize> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &self[pd_idx];

		if pde.is_4m() {
			return pde
				.flag()
				.contains(PageFlag::Present)
				.then(|| pde.addr() + pt_idx * PAGE_SIZE);
		} else {
			let pt = unsafe { (phys_to_virt(pde.addr()) as *mut PT).as_mut().unwrap() };
			let pte = pt[pt_idx];

			return pte.flag().contains(PageFlag::Present).then(|| pte.addr());
		}
	}

	fn addr_to_index(vaddr: usize) -> (usize, usize) {
		let pd_idx = vaddr / PT_COVER_SIZE;
		let pt_idx = (vaddr % PT_COVER_SIZE) / PAGE_SIZE;

		(pd_idx, pt_idx)
	}
}

impl Deref for PD {
	type Target = [PDE; 1024];

	fn deref(&self) -> &Self::Target {
		&self.entries
	}
}

impl DerefMut for PD {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.entries
	}
}
