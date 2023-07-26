use bitflags::bitflags;

bitflags! {
	#[derive(Debug, Clone, Copy)]
	pub struct SigFlag: u32 {
		const NoChildStop = 1;
		const OnStack = 2;
		const ResetHand = 4;
		const Restart = 8;
		const SigInfo = 16;
		const NoChildWait = 32;
		const NoDefer = 64;
	}
}
