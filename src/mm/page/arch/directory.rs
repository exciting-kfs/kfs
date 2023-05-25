use super::{util::invalidate_all_tlb, PageFlag, PT, PTE};
use crate::mm::{constant::*, util::*};
use crate::sync::singleton::Singleton;
use core::alloc::AllocError;
use core::ops::{Index, IndexMut};

extern "C" {
	pub static mut GLOBAL_PD_VIRT: PD;
}

pub static CURRENT_PD: Singleton<&mut PD> = Singleton::uninit();

#[repr(C, align(4096))]
pub struct PD {
	entries: [PDE; 1024],
}

impl PD {
	pub fn map_4m(&mut self, vaddr: usize, paddr: usize, flags: PageFlag) {
		let (pd_idx, _) = Self::addr_to_index(vaddr);

		self.entries[pd_idx] = PDE::new_4m(paddr, flags);
	}

	pub fn map_page(
		&mut self,
		vaddr: usize,
		paddr: usize,
		flags: PageFlag,
	) -> Result<(), AllocError> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self.entries[pd_idx];

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

		let pde = &mut self.entries[pd_idx];

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

		let pde = &self.entries[pd_idx];

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

impl Index<usize> for PD {
	type Output = PDE;

	fn index(&self, index: usize) -> &Self::Output {
		&self.entries[index]
	}
}

impl IndexMut<usize> for PD {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.entries[index]
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
