pub const TEMP_STACK_SIZE: usize = 0x80000;

#[repr(packed)]
#[allow(dead_code)]
pub struct TempStack {
	pub data: [u8; TEMP_STACK_SIZE],
}

impl TempStack {
	pub const fn new() -> Self {
		TempStack {
			data: [0; TEMP_STACK_SIZE],
		}
	}
}
