use bitflags::bitflags;

bitflags! {
	#[derive(Debug, Clone, Copy)]
	pub struct SigFlag: u32 {
		const NoCldStop = 1;
		const NoCldWait = 2;
		const SigInfo   = 4;
		const OnStack   = 0x08000000;
		const Restart   = 0x10000000;
		const NoDefer   = 0x40000000;
		const ResetHand = 0x80000000;
		const Restorer  = 0x04000000;
	}
}

impl SigFlag {
	pub const DEFAULT: SigFlag = Self::Restart;
}
