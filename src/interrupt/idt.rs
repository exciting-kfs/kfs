use core::{arch::asm, mem::size_of};

use crate::{
	interrupt::exception::CpuException,
	pr_info,
	sync::singleton::Singleton,
	x86::{DPL_USER, GDT},
};

use super::InterruptFrame;
use crate::x86::SystemDesc;

const IDTE_COUNT: usize = 256;

#[repr(align(8))]
pub struct IDT {
	entry: [SystemDesc; IDTE_COUNT],
}

impl IDT {
	pub const fn new() -> Self {
		Self {
			entry: [SystemDesc::new_null(); IDTE_COUNT],
		}
	}

	pub fn write_exception(&mut self, e: CpuException, entry: SystemDesc) {
		if e == CpuException::Reserved {
			panic!("idt: don't use the exception reserved for cpu.");
		}
		self.entry[e as usize] = entry;
	}

	pub fn write_interrupt(&mut self, index: usize, entry: SystemDesc) {
		if index < 32 || index >= IDTE_COUNT {
			panic!("idt: index out of range.");
		}
		self.entry[index] = entry;
	}

	pub fn load(&self) {
		let idtr = IDTR::new(self);
		unsafe {
			asm!(
				"lidt [{idtr_ptr}]",
				idtr_ptr = in(reg) &idtr
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
	const fn new(idt: &IDT) -> Self {
		IDTR {
			limit: (size_of::<IDT>() - 1) as u16,
			addr: idt,
		}
	}
}

extern "C" {
	fn handle_timer();
	fn handle_syscall();
	fn handle_keyboard();
	fn handle_divide_error();
	fn handle_invalid_opcode();
	fn handle_general_protection();
	fn handle_page_fault();
}

#[no_mangle]
pub extern "C" fn handle_syscall_impl(frame: InterruptFrame) {
	pr_info!("SYSCALL NO: {}, ARG1: {}", frame.eax, frame.ebx);
}

pub fn init() {
	let de = SystemDesc::new_interrupt(handle_divide_error as usize, GDT::KERNEL_CODE, DPL_USER);
	let ud = SystemDesc::new_interrupt(handle_invalid_opcode as usize, GDT::KERNEL_CODE, DPL_USER);
	let gp = SystemDesc::new_interrupt(
		handle_general_protection as usize,
		GDT::KERNEL_CODE,
		DPL_USER,
	);
	let pf = SystemDesc::new_interrupt(handle_page_fault as usize, GDT::KERNEL_CODE, DPL_USER);

	let keyboard = SystemDesc::new_interrupt(handle_keyboard as usize, GDT::KERNEL_CODE, DPL_USER);
	let lapic_timer = SystemDesc::new_interrupt(handle_timer as usize, GDT::KERNEL_CODE, DPL_USER);
	let syscall = SystemDesc::new_interrupt(handle_syscall as usize, GDT::KERNEL_CODE, DPL_USER);

	let mut idt = IDT.lock();
	idt.write_exception(CpuException::DE, de);
	idt.write_exception(CpuException::PF, pf);
	idt.write_exception(CpuException::UD, ud);
	idt.write_exception(CpuException::GP, gp);

	idt.write_interrupt(0x21, keyboard);
	idt.write_interrupt(0x22, lapic_timer);
	idt.write_interrupt(0x80, syscall);

	idt.load();
}
