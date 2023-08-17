/// Physical Region Descriptor.
#[repr(C)]
pub struct PRD {
	paddr: u32,
	conf: u32,
}

impl PRD {
	pub fn new(paddr: usize, byte_count: u16, eot: bool) -> Self {
		let eot = if eot { 1 << 31 } else { 0 };
		let conf = (byte_count as u32) | eot;
		Self {
			paddr: (paddr) as u32,
			conf,
		}
	}
}
