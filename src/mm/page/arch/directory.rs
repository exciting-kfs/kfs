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

pub static KERNEL_PD: Singleton<PD> = Singleton::uninit();

pub struct PD {
	inner: NonNull<[PDE; 1024]>,
}

impl PD {
	pub fn new(inner: NonNull<[PDE; 1024]>) -> PD {
		PD { inner }
	}

	/// Safety: self.inner is allocated from `Self::new()` or from outside
	fn inner_mut(&mut self) -> &mut [PDE; 1024] {
		unsafe { self.inner.as_mut() }
	}

	fn inner(&self) -> &[PDE; 1024] {
		unsafe { self.inner.as_ref() }
	}

	pub fn clone(&self) -> Result<Self, AllocError> {
		unsafe {
			let mut pd: NonNull<[PDE; 1024]> = alloc_pages(0, Zone::Normal)?.cast();

			for i in 0..1024 {
				let dst = addr_of_mut!(pd.as_mut()[i]);
				let src = &self.inner()[i];

				match src.clone() {
					Ok(copied) => dst.write(copied),
					Err(e) => {
						Self::clone_fail_cleanup(pd, i);
						return Err(e);
					}
				}
			}

			Ok(Self { inner: pd })
		}
	}

	fn clone_fail_cleanup(mut pd: NonNull<[PDE; 1024]>, failed_index: usize) {
		unsafe {
			for i in 0..failed_index {
				let pde = addr_of_mut!(pd.as_mut()[i]);
				(*pde).destory();
			}
		}
		free_pages(pd.cast())
	}

	pub fn map_4m(&mut self, vaddr: usize, paddr: usize, flags: PageFlag) {
		let (pd_idx, _) = Self::addr_to_index(vaddr);

		self.inner_mut()[pd_idx] = PDE::new_4m(paddr, flags);
	}

	pub fn map_page(
		&mut self,
		vaddr: usize,
		paddr: usize,
		flags: PageFlag,
	) -> Result<(), AllocError> {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self.inner_mut()[pd_idx];

		let pt = if pde.is_4m() {
			let pt = PT::new_from_4m(*pde)?;

			*pde = PDE::new(
				virt_to_phys(pt as *mut PT as usize),
				PageFlag::User | PageFlag::Write | PageFlag::Present,
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

		let pde = &mut self.inner_mut()[pd_idx];

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

		let pde = &self.inner()[pd_idx];

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
		&self.inner()[index]
	}
}

impl IndexMut<usize> for PD {
	fn index_mut(&mut self, index: usize) -> &mut Self::Output {
		&mut self.inner_mut()[index]
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

		let pt: NonNull<PT> = alloc_pages(0, Zone::Normal)?.cast();
		let src = self.as_pt().unwrap();
		unsafe { pt.as_ptr().copy_from_nonoverlapping(src, 1) }

		Ok(Self::new(virt_to_phys(pt.as_ptr() as usize), self.flag()))
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
			free_pages(unsafe { NonNull::new_unchecked(phys_to_virt(self.addr()) as *mut u8) });
		}
	}
}

impl AsMut<PageFlag> for PDE {
	fn as_mut(&mut self) -> &mut PageFlag {
		&mut self.data
	}
}
