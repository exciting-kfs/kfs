use super::{sig_flag::SigFlag, sig_mask::SigMask, sig_num::SigNum};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SigAction {
	handler: usize,
	flag: SigFlag,
	restorer: usize,
	mask2: SigMask,
	mask: SigMask,
}

impl SigAction {
	pub const fn empty() -> Self {
		Self {
			handler: 0,
			flag: SigFlag::empty(),
			restorer: 0,
			mask: SigMask::empty(),
			mask2: SigMask::empty(),
		}
	}

	pub fn new(addr: usize, mask: SigMask, flag: SigFlag) -> Self {
		Self {
			handler: addr,
			flag,
			restorer: 0,
			mask,
			mask2: SigMask::empty(),
		}
	}

	pub fn handler(&self) -> usize {
		self.handler
	}

	pub fn mask(&self) -> SigMask {
		self.mask
	}

	pub fn flag(&self) -> SigFlag {
		self.flag
	}
}

#[derive(Debug, Clone)]
pub enum SigHandler {
	Core,
	Continue,
	Stop,
	Terminate,
	Ignore,
	Some(SigAction),
}

impl SigHandler {
	pub fn some(act: SigAction) -> Self {
		Self::Some(act)
	}

	pub fn default(sig_num: SigNum) -> Self {
		use SigHandler::*;
		use SigNum::*;
		match sig_num {
			ABRT => Core,
			ALRM => Terminate,
			BUS => Core,
			CHLD => Ignore,
			CONT => Continue,
			FPE => Core,
			HUP => Terminate,
			ILL => Core,
			INT => Terminate,
			IO => Terminate,
			KILL => Terminate,
			PIPE => Terminate,
			PROF => Terminate,
			PWR => Terminate,
			QUIT => Core,
			SEGV => Core,
			STKFLT => Terminate,
			STOP => Stop,
			TSTP => Stop,
			SYS => Core,
			TERM => Terminate,
			TRAP => Core,
			TTIN => Stop,
			TTOU => Stop,
			URG => Ignore,
			USR1 => Terminate,
			USR2 => Terminate,
			VTALRM => Terminate,
			XCPU => Core,
			XFSZ => Core,
			WINCH => Ignore,
		}
	}

	pub fn is_default(&self) -> bool {
		match self {
			SigHandler::Some(_) => false,
			_ => true,
		}
	}

	pub fn get_flag(&self) -> SigFlag {
		match self {
			SigHandler::Some(act) => act.flag,
			_ => SigFlag::DEFAULT,
		}
	}
}
