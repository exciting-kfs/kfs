#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct InterruptInfo {
	instruction_pointer: usize,
	code_segment: usize,
	cpu_flags: usize,
	stack_pointer: usize,
	stack_segment: usize,
}
