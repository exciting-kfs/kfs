use core::alloc::{AllocError, Allocator, Layout};

use core::iter::repeat_with;
use core::mem::MaybeUninit;
use core::ops::Range;
use core::ptr::NonNull;

use core::slice::from_raw_parts;

use crate::mm::alloc::{Zone, PAGE_ALLOC};
use crate::mm::init::GLOBAL_PD_VIRT;
use crate::mm::page::{metapage_let, MetaPage, MetaPageTable, META_PAGE_TABLE};
use crate::mm::page::{PageFlag, PD};
use crate::mm::util::virt_to_phys;
use crate::mm::{constant::*, util::*};
use crate::pr_err;
use crate::sync::singleton::Singleton;

use super::AddressTree;

enum VMallocError {
	OutOfAddress,
	OutOfMemory(usize),
}

static ADDRESS_TREE: Singleton<AddressTree> = Singleton::uninit();
pub static VMALLOC: VMemAlloc = VMemAlloc;
pub struct VMemAlloc;

impl VMemAlloc {
	pub fn init(&self, area: Range<usize>) {
		unsafe { ADDRESS_TREE.write(AddressTree::new(area)) };
	}

	pub fn size(&self, ptr: NonNull<u8>) -> usize {
		metapage_let![dummy];

		let page = NonNull::from(
			&mut META_PAGE_TABLE.lock()
				[addr_to_pfn(self.global_pd().lookup(ptr.as_ptr() as usize).unwrap())],
		);
		dummy.push(page);

		let count = dummy.into_iter().count();

		dummy.disjoint();

		count * PAGE_SIZE
	}

	fn global_pd(&self) -> &mut PD {
		unsafe { &mut GLOBAL_PD_VIRT }
	}

	fn try_allocate(
		&self,
		head: &mut MetaPage,
		pages: usize,
	) -> Result<NonNull<[u8]>, VMallocError> {
		let base_address = ADDRESS_TREE
			.lock()
			.alloc(pages)
			.ok_or_else(|| VMallocError::OutOfAddress)?;

		for vaddr in (0..pages).map(|x| base_address + x * PAGE_SIZE) {
			let paddr = virt_to_phys(unsafe {
				PAGE_ALLOC
					.lock()
					.alloc_page(0, Zone::High)
					.map_err(|_| VMallocError::OutOfMemory(base_address))?
					.as_mut()
					.as_ptr() as usize
			});

			head.push(NonNull::from(
				&mut META_PAGE_TABLE.lock()[addr_to_pfn(paddr)],
			));

			self.global_pd()
				.map_page(
					vaddr,
					paddr,
					PageFlag::Present | PageFlag::Global | PageFlag::Write,
				)
				.map_err(|_| VMallocError::OutOfMemory(base_address))?;
		}

		Ok(NonNull::from(unsafe {
			from_raw_parts(base_address as *const u8, pages * PAGE_SIZE)
		}))
	}

	fn layout_to_pages(layout: Layout) -> usize {
		(layout.size() + PAGE_SIZE - 1) / PAGE_SIZE
	}

	fn free_pages(&self, head: &mut MetaPage, base_addr: usize, pages: usize) {
		for (i, entry) in repeat_with(|| head.pop()).map_while(|v| v).enumerate() {
			let page = MetaPageTable::metapage_to_ptr(entry);
			PAGE_ALLOC.lock().free_page(page);

			let addr = base_addr + i * PAGE_SIZE;
			let _ = self.global_pd().unmap_page(addr);
		}

		ADDRESS_TREE.lock().dealloc(base_addr, pages);
	}
}

unsafe impl Allocator for VMemAlloc {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let pages = Self::layout_to_pages(layout);

		metapage_let![dummy];
		let result = self.try_allocate(dummy, pages);

		if let Err(VMallocError::OutOfMemory(mem)) = result {
			self.free_pages(dummy, mem, pages);
		}

		dummy.disjoint();

		result.map_err(|_| AllocError)
	}

	unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
		let virt_base_addr = ptr.as_ptr() as usize;
		let phys_base_addr = match self.global_pd().lookup(virt_base_addr) {
			Some(x) => x,
			None => {
				pr_err!(
					"VirtualAllocator: Deallocation requested with not allocated pointer({:p})",
					ptr,
				);
				return;
			}
		};

		metapage_let![dummy];

		dummy.push(NonNull::from(
			&mut META_PAGE_TABLE.lock()[addr_to_pfn(phys_base_addr)],
		));

		let pages = Self::layout_to_pages(layout);
		self.free_pages(dummy, virt_base_addr, pages)
	}
}
