use bitflags::bitflags;

bitflags! {
	#[derive(Debug, Clone, Copy)]
	pub struct SigFlag: u32 {
		const OnStack = 1;
		const Restart = 2;
		const ResetHand = 4;
		const NoChildStop = 8;
		const NoDefer = 16;
		const NoChildWait = 32;
		const SigInfo = 64;

	}
}

impl SigFlag {
	pub const DEFAULT: SigFlag = Self::Restart;
}
