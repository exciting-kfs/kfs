use super::{sig_flag::SigFlag, sig_mask::SigMask, sig_num::SigNum};

#[derive(Debug, Clone)]
pub enum SigHandler {
	Core,
	Continue,
	Stop,
	Terminate,
	Ignore,
	Some(SigAction),
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SigAction {
	addr: usize,
	addr_info: usize,
	mask: SigMask,
	flag: SigFlag,
}

impl SigAction {
	pub const fn empty() -> Self {
		Self {
			addr: 0,
			addr_info: 0,
			mask: SigMask::empty(),
			flag: SigFlag::empty(),
		}
	}

	pub fn new(addr: usize, mask: SigMask, flag: SigFlag) -> Self {
		let (addr, addr_info) = match flag.contains(SigFlag::SigInfo) {
			true => (0, addr),
			false => (addr, 0),
		};

		Self {
			addr,
			addr_info,
			mask,
			flag,
		}
	}

	pub fn handler(&self) -> usize {
		match self.flag.contains(SigFlag::SigInfo) {
			true => self.addr_info,
			false => self.addr,
		}
	}

	pub fn mask(&self) -> SigMask {
		self.mask
	}

	pub fn flag(&self) -> SigFlag {
		self.flag
	}
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
			IOT => Core,
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
}
