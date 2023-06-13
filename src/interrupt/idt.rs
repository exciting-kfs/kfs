use core::mem::size_of;

use crate::{interrupt::exception::CpuException, sync::singleton::Singleton};

use super::{
	exception::{divide_error, general_protection, page_fault, undefined},
	hw::{keyboard, lvt_error, timer},
	idte::IDTE,
};

const IDTE_COUNT: usize = 256;

#[repr(align(8))]
pub struct IDT {
	entry: [IDTE; IDTE_COUNT],
}

impl IDT {
	pub const fn new() -> Self {
		Self {
			entry: [IDTE::null(); IDTE_COUNT],
		}
	}

	pub fn write_exception(&mut self, e: CpuException, entry: IDTE) {
		if e == CpuException::Reserved {
			panic!("idt: don't use the exception reserved for cpu.");
		}
		self.entry[e as usize] = entry
	}

	pub fn write_interrupt(&mut self, index: usize, entry: IDTE) {
		if index < 32 || index >= IDTE_COUNT {
			panic!("idt: index out of range.");
		}
		self.entry[index] = entry
	}

	pub fn load(&self) {
		let size: u16 = (IDTE_COUNT * size_of::<IDTE>() - 1) as u16;
		let idtr_ptr = IDTR::new(size, unsafe { IDT.as_mut_ptr() });

		unsafe {
			core::arch::asm!(
				"lidt [{0}]",
				in(reg) &idtr_ptr
			);
		}
	}
}

static IDT: Singleton<IDT> = Singleton::new(IDT::new());

#[repr(packed)]
struct IDTR {
	limit: u16,
	addr: *const IDT,
}

impl IDTR {
	const fn new(limit: u16, addr: *const IDT) -> Self {
		IDTR { limit, addr }
	}
}

pub fn init() {
	let de = IDTE::interrupt_kernel(divide_error::handler as usize);
	let ud = IDTE::interrupt_kernel(undefined::handler as usize);
	let gp = IDTE::interrupt_kernel(general_protection::handler as usize);
	let pf = IDTE::interrupt_kernel(page_fault::handler as usize);
	let kb = IDTE::interrupt_kernel(keyboard::handler as usize);
	let tm = IDTE::interrupt_kernel(timer::handler as usize);
	let le = IDTE::interrupt_kernel(lvt_error::handler as usize);

	let mut idt = IDT.lock();
	idt.write_exception(CpuException::DE, de);
	idt.write_exception(CpuException::PF, pf);
	idt.write_exception(CpuException::UD, ud);
	idt.write_exception(CpuException::GP, gp);

	idt.write_interrupt(0x21, kb);
	idt.write_interrupt(0x22, tm);
	idt.write_interrupt(0xfe, le);

	idt.load();
}
