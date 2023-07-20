use crate::{
	mm::{alloc::virt::AddressSpace, util::virt_to_phys},
	process::task::CURRENT,
};

pub mod arch;
pub mod bitrange;
pub mod lazy_constant;
pub mod lcg;

pub struct LazyInit<T> {
	value: Option<T>,
	init: fn() -> T,
}

impl<T> LazyInit<T> {
	pub const fn new(cb: fn() -> T) -> Self {
		LazyInit {
			value: None,
			init: cb,
		}
	}

	pub fn get(&mut self) -> &mut T {
		if let None = self.value {
			self.value = Some((self.init)())
		}
		self.value.as_mut().unwrap()
	}
}

#[derive(Clone, Copy)]
pub struct Vaddr<T>(*mut T);

impl<T> Vaddr<T> {
	pub fn from_raw(ptr: *mut T) -> Self {
		Vaddr(ptr)
	}

	pub fn as_usize(&self) -> usize {
		self.0 as usize
	}

	pub fn paddr(&self) -> Option<Paddr<T>> {
		let vaddr = self.0 as usize;
		let paddr = match AddressSpace::identify(vaddr) {
			AddressSpace::User => unsafe {
				CURRENT.get_mut().lock_memory()?.get_pd().lookup(vaddr)
			},
			_ => Some(virt_to_phys(vaddr)),
		};
		paddr.map(|addr| Paddr(addr as *mut T))
	}
}

#[derive(Clone, Copy)]
pub struct Paddr<T>(*mut T);

impl<T> Paddr<T> {
	pub fn from_raw(ptr: *mut T) -> Self {
		Paddr(ptr)
	}

	pub fn as_usize(&self) -> usize {
		self.0 as usize
	}
}
