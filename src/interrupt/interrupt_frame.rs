use core::fmt;

use crate::x86::{get_eflags, DPL_USER, GDT};

/// Stack Frame after interrupt.
/// constructed by src/asm/interrupt.S (handle_interrupt)
#[repr(C)]
#[derive(Debug, Clone)]
pub struct InterruptFrame {
	pub ebp: usize,
	pub edi: usize,
	pub esi: usize,
	pub edx: usize,
	pub ecx: usize,
	pub ebx: usize,
	pub eax: usize,
	pub ds: usize,
	pub es: usize,
	pub fs: usize,
	pub gs: usize,

	// Additional informations
	pub handler: usize,
	pub error_code: usize,

	// Fields bellow here are managed by CPU
	pub eip: usize,
	pub cs: usize,
	pub eflags: usize,

	// Valid only when stack switching was occured. (CPL 3 => CPL 1)
	pub esp: usize,
	pub ss: usize,
}

impl InterruptFrame {
	pub fn new_user(user_return_addr: usize, user_stack: usize) -> Self {
		let eflags = get_eflags() | (1 << 9); // enable interrupt

		Self {
			ebp: 0,
			edi: 0,
			esi: 0,
			edx: 0,
			ecx: 0,
			ebx: 0,
			eax: 0,
			ds: GDT::USER_DATA | DPL_USER,
			es: GDT::USER_DATA | DPL_USER,
			fs: GDT::USER_DATA | DPL_USER,
			gs: GDT::USER_DATA | DPL_USER,
			handler: 0,
			error_code: 0,
			eip: user_return_addr,
			cs: GDT::USER_CODE | DPL_USER,
			eflags,
			esp: user_stack,
			ss: GDT::USER_DATA | DPL_USER,
		}
	}

	pub fn is_user(&self) -> bool {
		(self.cs & 0x0000fffc) == GDT::USER_CODE
	}
}

impl fmt::Display for InterruptFrame {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			concat!(
				"[STACK REG]\n",
				" ESP: {:#010x}\n",
				" EBP: {:#010x}\n",
				"[PROGRAM COUNTER REG]\n",
				" EIP: {:#010x}\n",
				"[GENERAL PURPOSE REG]\n",
				" EAX: {:#010x}\n",
				" EBX: {:#010x}\n",
				" ECX: {:#010x}\n",
				" EDX: {:#010x}\n",
				" EDI: {:#010x}\n",
				" ESI: {:#010x}\n",
				"[SEGMENT SELECTOR]\n",
				" CS: {}\n",
				" SS: {}\n",
				" DS: {}\n",
				" ES: {}\n",
				" FS: {}\n",
				" GS: {}\n",
				"[EXTRA]\n",
				" EFLAGS: {:032b}\n",
				" HANDLER: {:#010x}\n",
				" ERROR_CODE: {:#010x}"
			),
			self.esp,
			self.ebp,
			self.eip,
			self.eax,
			self.ebx,
			self.ecx,
			self.edx,
			self.edi,
			self.esi,
			self.cs & 0x0000ffff,
			self.ss & 0x0000ffff,
			self.ds & 0x0000ffff,
			self.es & 0x0000ffff,
			self.fs & 0x0000ffff,
			self.gs & 0x0000ffff,
			self.eflags,
			self.handler,
			self.error_code,
		)
	}
}
