use core::alloc::{AllocError, Allocator, Layout};

use core::iter::repeat_with;
use core::mem::MaybeUninit;
use core::ops::Range;
use core::ptr::NonNull;

use crate::mm::alloc::{page, Zone};
use crate::mm::page::{index_to_meta, PageFlag, KERNEL_PD};
use crate::mm::page::{meta_to_ptr, metapage_let, MetaPage};
use crate::mm::util::virt_to_phys;
use crate::mm::{constant::*, util::*};
use crate::sync::Locked;
use core::slice::from_raw_parts;

use super::AddressTree;

enum VMallocError {
	OutOfAddress,
	OutOfMemory(usize),
}

pub static ADDRESS_TREE: Locked<MaybeUninit<AddressTree>> = Locked::uninit();

pub static VMALLOC: VMemAlloc = VMemAlloc;

/// Virtual memory allocator
/// Unlike Physical memory allocator, this allocates virtually continuous memory.
pub struct VMemAlloc;

impl VMemAlloc {
	pub fn init(&self, area: Range<usize>) {
		let mut addr_tree = ADDRESS_TREE.lock();

		unsafe { addr_tree.as_mut_ptr().write(AddressTree::new(area)) };
	}

	pub fn size(&self, ptr: NonNull<u8>) -> usize {
		metapage_let![dummy];

		let paddr = KERNEL_PD
			.lookup(ptr.as_ptr() as usize)
			.expect("BUG: not allocated with vmalloc");
		let mut page = index_to_meta(addr_to_pfn(paddr));
		unsafe { page.as_mut().push(NonNull::new_unchecked(dummy)) };

		let count = dummy.into_iter().count();

		dummy.disjoint();

		count * PAGE_SIZE
	}

	fn try_allocate(
		&self,
		head: &mut MetaPage,
		pages: usize,
	) -> Result<NonNull<[u8]>, VMallocError> {
		let base_address = unsafe {
			ADDRESS_TREE
				.lock()
				.assume_init_mut()
				.alloc(pages)
				.ok_or_else(|| VMallocError::OutOfAddress)?
		};

		for vaddr in (0..pages).map(|x| base_address + x * PAGE_SIZE) {
			let paddr = virt_to_phys(unsafe {
				page::alloc_pages(0, Zone::High)
					.map_err(|_| VMallocError::OutOfMemory(base_address))?
					.as_mut()
					.as_ptr() as usize
			});

			head.push(index_to_meta(addr_to_pfn(paddr)));

			KERNEL_PD.map_kernel(
				vaddr,
				paddr,
				PageFlag::Present | PageFlag::Global | PageFlag::Write,
			);
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
			KERNEL_PD.unmap_kernel(addr);
		}

		unsafe {
			ADDRESS_TREE
				.lock()
				.assume_init_mut()
				.dealloc(base_addr, pages)
		};
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
		let phys_base_addr = KERNEL_PD
			.lookup(virt_base_addr)
			.expect("BUG: not allocated with vmalloc");

		metapage_let![dummy];

		let mut meta = index_to_meta(addr_to_pfn(phys_base_addr));

		meta.as_mut().push(NonNull::new_unchecked(dummy));

		let pages = Self::layout_to_pages(layout);
		self.free_pages(dummy, virt_base_addr, pages)
	}
}
