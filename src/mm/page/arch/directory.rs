use super::{util::invalidate_all_tlb, PageFlag, PT, PTE};
use crate::mm::alloc::page::{alloc_pages, free_pages};
use crate::mm::alloc::Zone;
use crate::mm::{constant::*, util::*};
use crate::sync::singleton::Singleton;
use core::alloc::AllocError;
use core::ops::{Index, IndexMut};
use core::ptr::{addr_of_mut, NonNull};

extern "C" {
	pub static mut GLOBAL_PD_VIRT: [PDE; 1024];
}

pub static CURRENT_PD: Singleton<PD> = Singleton::uninit();

fn alloc_one_page() -> Result<*mut u8, AllocError> {
	unsafe { Ok(alloc_pages(0, Zone::Normal)?.as_mut().as_mut_ptr()) }
}

fn free_one_page(page: *mut u8) {
	free_pages(NonNull::new(page).unwrap());
}

pub struct PD<'a> {
	inner: &'a mut [PDE; 1024],
}

impl<'a> PD<'a> {
	pub fn new(inner: &mut [PDE; 1024]) -> PD<'_> {
		PD { inner }
	}

	pub fn clone(&self) -> Result<Self, AllocError> {
		unsafe {
			let pd: *mut [PDE; 1024] = alloc_one_page()?.cast();

			for i in 0..1024 {
				let dst = addr_of_mut!((*pd)[i]);
				let src = &self.inner[i];

				match src.clone() {
					Ok(copied) => dst.write(copied),
					Err(e) => {
						Self::clone_fail_cleanup(pd, i);
						return Err(e);
					}
				}
			}

			Ok(Self { inner: &mut *pd })
		}
	}

	fn clone_fail_cleanup(pd: *mut [PDE; 1024], failed_index: usize) {
		unsafe {
			for i in 0..failed_index {
				let pde = addr_of_mut!((*pd)[i]);
				(*pde).destory();
			}
		}
		free_one_page(pd.cast())
	}

	pub fn map_4m(&mut self, vaddr: usize, paddr: usize, flags: PageFlag) {
		let (pd_idx, _) = Self::addr_to_index(vaddr);

		self.inner[pd_idx] = PDE::new_4m(paddr, flags);
	}

	pub fn map_page(
		&mut self,
		vaddr: usize,
		paddr: usize,
		flags: PageFlag,
	) -> Result<(), AllocError> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self.inner[pd_idx];

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

		let pde = &mut self.inner[pd_idx];

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

		let pde = &self.inner[pd_idx];

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

impl<'a> Index<usize> for PD<'a> {
	type Output = PDE;

	fn index(&self, index: usize) -> &Self::Output {
		&self.inner[index]
	}
}

impl<'a> IndexMut<usize> for PD<'a> {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.inner[index]
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

	pub fn clone(&self) -> Result<Self, AllocError> {
		if self.is_4m() {
			return Ok(Self { data: self.data });
		}

		let pt: *mut PT = alloc_one_page()?.cast();
		let src = self.as_pt().unwrap();
		unsafe { pt.copy_from_nonoverlapping(src, PAGE_SIZE) }

		Ok(Self::new(virt_to_phys(pt as usize), self.flag()))
	}

	pub fn new_4m(paddr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain((paddr as u32 & Self::ADDR_MASK_4M) | Self::PSE)
				| flags,
		}
	}

	pub fn new(paddr: usize, flags: PageFlag) -> Self {
		Self {
			data: PageFlag::from_bits_retain(paddr as u32 & Self::ADDR_MASK) | flags,
		}
	}

	pub fn is_4m(&self) -> bool {
		(self.data.bits() & Self::PSE) != 0
	}

	pub fn as_pt(&self) -> Option<&PT> {
		if self.is_4m() {
			return None;
		}
		unsafe { (phys_to_virt(self.addr()) as *const PT).as_ref() }
	}

	pub fn addr(&self) -> usize {
		(self.data.bits() & Self::ADDR_MASK) as usize
	}

	pub fn flag(&self) -> PageFlag {
		PageFlag::from_bits_truncate(self.data.bits())
	}

	pub fn destory(self) {
		if !self.is_4m() {
			free_one_page(phys_to_virt(self.addr()) as *mut u8);
		}
	}
}

impl AsMut<PageFlag> for PDE {
	fn as_mut(&mut self) -> &mut PageFlag {
		&mut self.data
	}
}
