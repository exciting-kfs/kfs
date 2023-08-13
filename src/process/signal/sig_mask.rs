use bitflags::bitflags;

use super::sig_num::SigNum;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug)]
	pub struct SigMask: u32 {
		const HUP = (1 << (SigNum::HUP as u32 - 1));
		const INT = (1 << (SigNum::INT as u32 - 1));
		const QUIT = (1 << (SigNum::QUIT as u32 - 1));
		const ILL = (1 << (SigNum::ILL as u32 - 1));
		const TRAP = (1 << (SigNum::TRAP as u32 - 1));
		const ABRT = (1 << (SigNum::ABRT as u32 - 1));
		const BUS = (1 << (SigNum::BUS as u32 - 1));
		const FPE = (1 << (SigNum::FPE as u32 - 1));
		const KILL = (1 << (SigNum::KILL as u32 - 1));
		const USR1 = (1 << (SigNum::USR1 as u32 - 1));
		const SEGV = (1 << (SigNum::SEGV as u32 - 1));
		const USR2 = (1 << (SigNum::USR2 as u32 - 1));
		const PIPE = (1 << (SigNum::PIPE as u32 - 1));
		const ALRM = (1 << (SigNum::ALRM as u32 - 1));
		const TERM = (1 << (SigNum::TERM as u32 - 1));
		const STKFLT = (1 << (SigNum::STKFLT as u32 - 1));
		const CHLD = (1 << (SigNum::CHLD as u32 - 1));
		const CONT = (1 << (SigNum::CONT as u32 - 1));
		const STOP = (1 << (SigNum::STOP as u32 - 1));
		const TSTP = (1 << (SigNum::TSTP as u32 - 1));
		const TTIN = (1 << (SigNum::TTIN as u32 - 1));
		const TTOU = (1 << (SigNum::TTOU as u32 - 1));
		const URG = (1 << (SigNum::URG as u32 - 1));
		const XCPU = (1 << (SigNum::XCPU as u32 - 1));
		const XFSZ = (1 << (SigNum::XFSZ as u32 - 1));
		const VTALRM = (1 << (SigNum::VTALRM as u32 - 1));
		const PROF = (1 << (SigNum::PROF as u32 - 1));
		const WINCH = (1 << (SigNum::WINCH as u32 - 1));
		const IO = (1 << (SigNum::IO as u32 - 1));
		const PWR = (1 << (SigNum::PWR as u32 - 1));
		const SYS = (1 << (SigNum::SYS as u32 - 1));
	}
}

impl From<SigNum> for SigMask {
	fn from(value: SigNum) -> Self {
		SigMask::from_bits_truncate(1 << (value as u32 - 1))
	}
}
