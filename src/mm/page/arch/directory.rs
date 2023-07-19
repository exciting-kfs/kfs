use super::{PageFlag, PT, PTE};
use crate::mm::alloc::page::{alloc_pages, free_pages};
use crate::mm::alloc::virt::AddressSpace;
use crate::mm::alloc::Zone;
use crate::mm::{constant::*, util::*};
use crate::sync::singleton::Singleton;

use core::alloc::AllocError;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::ptr::NonNull;

extern "C" {
	pub static mut GLOBAL_PD_VIRT: [PDE; 1024];
}

const NR_VMALLOC_PT: usize = (KMAP_OFFSET - VMALLOC_OFFSET) / PT_COVER_SIZE;
pub static VMALLOC_PT: Singleton<[PT; NR_VMALLOC_PT]> = Singleton::new([PT::new(); NR_VMALLOC_PT]);

const NR_KMAP_PT: usize = (HIGH_IO_OFFSET - KMAP_OFFSET) / PT_COVER_SIZE;
pub static KMAP_PT: Singleton<[PT; NR_KMAP_PT]> = Singleton::new([PT::new(); NR_KMAP_PT]);

pub static KERNEL_PD: PD = PD::uninit();

#[repr(transparent)]
pub struct PD {
	inner: UnsafeCell<NonNull<[PDE; 1024]>>,
}

unsafe impl Sync for PD {}

impl PD {
	pub const fn uninit() -> PD {
		PD {
			inner: UnsafeCell::new(NonNull::dangling()),
		}
	}

	pub fn init(&self, inner: NonNull<[PDE; 1024]>) {
		unsafe { self.inner.get().write(inner) };
	}

	pub fn new() -> Result<Self, AllocError> {
		unsafe {
			let pd: NonNull<[PDE; 1024]> = alloc_pages(0, Zone::Normal)?.cast();

			pd.as_ptr().copy_from_nonoverlapping(KERNEL_PD.inner(), 1);

			Ok(Self {
				inner: UnsafeCell::new(pd),
			})
		}
	}

	/// Safety: self.inner is allocated from `Self::new()` or from outside
	fn inner_mut(&mut self) -> &mut [PDE; 1024] {
		unsafe { (*self.inner.get()).as_mut() }
	}

	fn inner(&self) -> &[PDE; 1024] {
		unsafe { (*self.inner.get()).as_ref() }
	}

	fn map_kmap_area(&self, vaddr: usize, paddr: usize, flags: PageFlag) {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr - KMAP_OFFSET);

		KMAP_PT.lock()[pd_idx][pt_idx] = PTE::new(paddr, flags);
	}

	fn map_vmalloc_area(&self, vaddr: usize, paddr: usize, flags: PageFlag) {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr - VMALLOC_OFFSET);

		VMALLOC_PT.lock()[pd_idx][pt_idx] = PTE::new(paddr, flags);
	}

	fn unmap_kmap_area(&self, vaddr: usize) {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr - KMAP_OFFSET);

		KMAP_PT.lock()[pd_idx][pt_idx] = PTE::new(0, PageFlag::Global);
	}

	fn unmap_vmalloc_area(&self, vaddr: usize) {
		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr - VMALLOC_OFFSET);

		VMALLOC_PT.lock()[pd_idx][pt_idx] = PTE::new(0, PageFlag::Global);
	}

	pub fn map_kernel(&self, vaddr: usize, paddr: usize, flags: PageFlag) {
		match AddressSpace::identify(vaddr) {
			AddressSpace::Kmap => self.map_kmap_area(vaddr, paddr, flags),
			AddressSpace::Vmalloc => self.map_vmalloc_area(vaddr, paddr, flags),
			_ => return,
		};
		invlpg(vaddr);
	}

	pub fn unmap_kernel(&self, vaddr: usize) {
		match AddressSpace::identify(vaddr) {
			AddressSpace::Kmap => self.unmap_kmap_area(vaddr),
			AddressSpace::Vmalloc => self.unmap_vmalloc_area(vaddr),
			_ => return,
		};
		invlpg(vaddr);
	}

	pub fn map_user(
		&mut self,
		vaddr: usize,
		paddr: usize,
		flags: PageFlag,
	) -> Result<(), AllocError> {
		match AddressSpace::identify(vaddr) {
			AddressSpace::User => (),
			_ => return Ok(()),
		};

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

		invlpg(vaddr);

		Ok(())
	}

	pub fn unmap_user(&mut self, vaddr: usize) {
		match AddressSpace::identify(vaddr) {
			AddressSpace::User => (),
			_ => return,
		};

		let (pd_idx, pt_idx) = Self::addr_to_index(vaddr);

		let pde = &mut self.inner_mut()[pd_idx];

		if pde.is_4m() {
			return;
		}

		let pt = unsafe { (phys_to_virt(pde.addr()) as *mut PT).as_mut().unwrap() };

		let mut new_flag = pt[pt_idx].flag();
		new_flag.remove(PageFlag::Present);

		pt[pt_idx].set_flag(new_flag);

		invlpg(vaddr);
	}

	fn lookup_arbitary(&self, vaddr: usize) -> Option<usize> {
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

	pub fn lookup(&self, vaddr: usize) -> Option<usize> {
		match AddressSpace::identify(vaddr) {
			AddressSpace::Kernel => Some(virt_to_phys(vaddr)),
			AddressSpace::HighIO => Some(vaddr),
			AddressSpace::Kmap => {
				let _lock = KMAP_PT.lock();
				self.lookup_arbitary(vaddr)
			}
			AddressSpace::Vmalloc => {
				let _lock = VMALLOC_PT.lock();
				self.lookup_arbitary(vaddr)
			}
			AddressSpace::User => self.lookup_arbitary(vaddr),
		}
	}

	fn addr_to_index(vaddr: usize) -> (usize, usize) {
		let pd_idx = vaddr / PT_COVER_SIZE;
		let pt_idx = (vaddr % PT_COVER_SIZE) / PAGE_SIZE;

		(pd_idx, pt_idx)
	}

	pub fn pick_up(&self) {
		let addr = virt_to_phys(self.inner() as *const _ as usize);

		unsafe { asm!("mov cr3, {pd}", pd = in(reg) addr) };
	}
}

impl Drop for PD {
	fn drop(&mut self) {
		let inner = self.inner_mut();
		for pde in inner.iter().take(VM_OFFSET / PT_COVER_SIZE) {
			if !pde.is_4m() {
				let vaddr = phys_to_virt(pde.addr());

				free_pages(unsafe { NonNull::new_unchecked(vaddr as *mut u8) });
			}
		}
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
}
