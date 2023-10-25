mod generic;
mod page_fault;

use crate::pr_info;

#[repr(u8)]
#[derive(PartialEq)]
pub enum CpuException {
	DE,  // Divide Error
	DB,  // Debug Exception
	NMI, // NonMaskable Interrupt
	BP,  // BreakPoint
	OF,  // Overflow
	BR,  // Bound Range Exceeded
	UD,  // Undefined Opcode
	NM,  // No Math Coprocessor
	DF,  // Double Fault
	CSO, // Coprocessor Segment Overrun
	TS,  // Invalid TSS
	NP,  // Segment Not Present
	SS,  // Stack Segment Fault
	GP,  // General Protection
	PF,  // Page Fault
	Reserved,
	MF, // Math Fault (x87 FPU Floating-Point Error)
	AC, // Alignment Check
	MC, // Machine Check
	XM, // SIMD Floating-Point Exception
	VE, // Virtualization Exception
	CP, // Control Protection Exception
}

impl CpuException {
	pub fn invoke(self) {
		match self {
			Self::DE => invoke_divide_error(),
			Self::PF => invoke_page_fault(),
			_ => {}
		}
	}
}

fn invoke_divide_error() {
	unsafe { core::arch::asm!("mov eax, 1", "mov ecx, 0", "div ecx") };
}

fn invoke_page_fault() {
	let ptr: *const usize = 0x0 as *const usize;
	let a = unsafe { *ptr };

	pr_info!("{}", a + 1);
}
