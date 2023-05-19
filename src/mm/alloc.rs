mod cache;
mod kmalloc;
mod page;
mod phys;
mod virt;

pub enum Zone {
	Normal,
	High,
}

pub enum GFP {
	Normal,
	Atomic,
}

pub use kmalloc::{kfree, kmalloc, ksize};
pub use page::{PageAlloc, PAGE_ALLOC};
pub use phys::{MemAtomic, MemNormal};
pub use virt::{vfree, vinit, vmalloc, vsize};
