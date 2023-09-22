use core::{alloc::AllocError, ptr::NonNull};

use crate::mm::page::{PageFlag, KERNEL_PD};
use crate::mm::{constant::*, util::*};
use crate::sync::Locked;

use super::AddressSpace;

static KMAP_BITMAP: Locked<BitMap> = Locked::new(BitMap::new());

struct BitMap {
	inner: [usize; 32],
}

impl BitMap {
	pub const fn new() -> Self {
		BitMap { inner: [0; 32] }
	}

	pub fn find_free_space(&self) -> Option<usize> {
		for (i, x) in self.inner.iter().enumerate() {
			let x = *x;
			if x != usize::MAX {
				return Some(i * 32 + x.trailing_ones() as usize);
			}
		}

		None
	}

	fn toggle_bitmap(&mut self, idx: usize) {
		let idx_h = idx / 32;
		let idx_l = idx % 32;

		self.inner[idx_h] ^= 1 << idx_l;
	}
}

pub fn kmap(paddr: usize) -> Result<NonNull<u8>, AllocError> {
	let mut bitmap = KMAP_BITMAP.lock();

	let idx = bitmap.find_free_space().ok_or(AllocError)?;
	bitmap.toggle_bitmap(idx);

	let vaddr = KMAP_OFFSET + pfn_to_addr(idx);

	KERNEL_PD.map_kernel(
		vaddr,
		paddr,
		PageFlag::Present | PageFlag::Write | PageFlag::Global,
	);

	// sefety: vaddr is at least `KMAP_OFFSET` (which is not null)
	Ok(unsafe { NonNull::new_unchecked(vaddr as *mut u8) })
}

pub fn kunmap(vaddr: usize) {
	// early return
	if !matches!(AddressSpace::identify(vaddr), AddressSpace::Kmap) {
		return;
	}

	let mut bitmap = KMAP_BITMAP.lock();

	let idx = addr_to_pfn(vaddr - KMAP_OFFSET);
	bitmap.toggle_bitmap(idx);

	KERNEL_PD.unmap_kernel(vaddr);
}

mod test {
	use crate::mm::alloc::{
		page::{alloc_pages, free_pages},
		Zone,
	};

	use super::*;
	use alloc::vec::Vec;
	use kfs_macro::ktest;

	#[ktest]
	pub fn simple() {
		let vaddr = alloc_pages(0, Zone::High).unwrap().as_ptr().cast::<u8>() as usize;

		let paddr = virt_to_phys(vaddr);

		let page = kmap(paddr).unwrap().as_ptr();

		// must not crash
		unsafe { page.write_bytes(42, PAGE_SIZE) };

		kunmap(page as usize);
		free_pages(unsafe { NonNull::new_unchecked(vaddr as *mut u8) });
	}

	#[ktest]
	pub fn repeat_map_unmap() {
		let pages = alloc_pages(MAX_RANK, Zone::High)
			.unwrap()
			.as_ptr()
			.cast::<u8>() as usize;

		let paddr = virt_to_phys(pages);

		let mut mapped_pages = Vec::new();
		let mut count = 0;

		// kmap while OOM
		while let Ok(page) = kmap(paddr + count * PAGE_SIZE) {
			unsafe { page.as_ptr().write_bytes(42, PAGE_SIZE) }
			mapped_pages.push(page);
			count += 1;
		}

		// kunmap all
		for p in mapped_pages.drain(..) {
			kunmap(p.as_ptr() as usize);
		}

		// re-kmap as many as before
		for i in 0..count {
			mapped_pages.push(kmap(paddr + i * PAGE_SIZE).unwrap());
		}

		for p in mapped_pages.drain(..) {
			kunmap(p.as_ptr() as usize);
		}

		free_pages(unsafe { NonNull::new_unchecked(pages as *mut u8) });
	}
}
