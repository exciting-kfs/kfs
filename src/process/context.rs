extern "C" {
	/// switch stack via exchange ESP
	/// see asm/interrupt.S
	pub fn switch_stack(prev_stack: &mut usize, next_stack: &mut usize);
}
