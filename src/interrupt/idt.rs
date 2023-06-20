use core::mem::size_of;

use crate::{interrupt::exception::CpuException, sync::singleton::Singleton};

use super::idte::IDTE;

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

extern "C" {
	fn handle_timer();
	fn handle_keyboard();
	fn handle_divide_error();
	fn handle_invalid_opcode();
	fn handle_general_protection();
	fn handle_page_fault();
}

pub fn init() {
	let de = IDTE::interrupt_kernel(handle_divide_error as usize);
	let ud = IDTE::interrupt_kernel(handle_invalid_opcode as usize);
	let gp = IDTE::interrupt_kernel(handle_general_protection as usize);
	let pf = IDTE::interrupt_kernel(handle_page_fault as usize);
	let kb = IDTE::interrupt_kernel(handle_keyboard as usize);
	let tm = IDTE::interrupt_kernel(handle_timer as usize);

	let mut idt = IDT.lock();
	idt.write_exception(CpuException::DE, de);
	idt.write_exception(CpuException::PF, pf);
	idt.write_exception(CpuException::UD, ud);
	idt.write_exception(CpuException::GP, gp);

	idt.write_interrupt(0x21, kb);
	idt.write_interrupt(0x22, tm);

	idt.load();
}
