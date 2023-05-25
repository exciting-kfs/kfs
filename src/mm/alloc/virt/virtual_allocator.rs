use core::alloc::{AllocError, Allocator, Layout};

use core::iter::repeat_with;
use core::ops::Range;
use core::ptr::NonNull;

use core::slice::from_raw_parts;

use crate::mm::alloc::{page, Zone};
use crate::mm::page::{index_to_meta, PageFlag};
use crate::mm::page::{map_page, meta_to_ptr, metapage_let, to_phys, unmap_page, MetaPage};
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

		let paddr = to_phys(ptr.as_ptr() as usize).unwrap();
		let page = index_to_meta(addr_to_pfn(paddr));
		dummy.push(page);

		let count = dummy.into_iter().count();

		dummy.disjoint();

		count * PAGE_SIZE
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
				page::alloc_pages(0, Zone::High)
					.map_err(|_| VMallocError::OutOfMemory(base_address))?
					.as_mut()
					.as_ptr() as usize
			});

			head.push(index_to_meta(addr_to_pfn(paddr)));

			map_page(
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
			let page = meta_to_ptr(entry);
			page::free_pages(page);
			let addr = base_addr + i * PAGE_SIZE;
			let _ = unmap_page(addr);
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
		let phys_base_addr = match to_phys(virt_base_addr) {
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

		dummy.push(index_to_meta(addr_to_pfn(phys_base_addr)));

		let pages = Self::layout_to_pages(layout);
		self.free_pages(dummy, virt_base_addr, pages)
	}
}
