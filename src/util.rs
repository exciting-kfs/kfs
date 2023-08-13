use crate::{pr_info, printk};

pub mod arch;
pub mod backtrace;
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

	// pub fn paddr(&self) -> Option<Paddr<T>> {
	// 	let vaddr = self.0 as usize;
	// 	let paddr = match AddressSpace::identify(vaddr) {
	// 		AddressSpace::User => unsafe {
	// 			CURRENT.get_mut().lock_memory()?.get_pd().lookup(vaddr)
	// 		},
	// 		_ => Some(virt_to_phys(vaddr)),
	// 	};
	// 	paddr.map(|addr| Paddr(addr as *mut T))
	// }
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

pub unsafe fn print_stack(esp: *const usize, count: usize) {
	pr_info!("[[stack]]");
	for i in 0..count {
		let next = esp.offset(i as isize);
		pr_info!("----------------------");
		pr_info!("{:x?}: 0x{:x}", next, *next);
	}
}

pub unsafe fn print_memory(ptr: *const u8, count: usize) {
	pr_info!("[[mem]]");
	pr_info!("range: {:x?} ~ {:x?}", ptr, ptr.offset(count as isize));
	let line = count / 16 + 1;

	for i in 0..line {
		printk!("0x{:x?}: ", ptr.offset((i * 16) as isize));

		for j in 0..16 {
			let next = ptr.offset((i * 16 + j) as isize);
			printk!("{:02x?} ", *next);
		}
		printk!("\n");
	}
}
