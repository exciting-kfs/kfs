use bitflags::bitflags;

use super::sig_num::SigNum;

bitflags! {
	#[repr(transparent)]
	#[derive(Clone, Copy, Debug)]
	pub struct SigFlag: u32 {
		const SIGHUP = (1 << (SigNum::SIGHUP as u32 - 1));
		const SIGINT = (1 << (SigNum::SIGINT as u32 - 1));
		const SIGQUIT = (1 << (SigNum::SIGQUIT as u32 - 1));
		const SIGILL = (1 << (SigNum::SIGILL as u32 - 1));
		const SIGTRAP = (1 << (SigNum::SIGTRAP as u32 - 1));
		const SIGABRT = (1 << (SigNum::SIGABRT as u32 - 1));
		const SIGIOT = (1 << (SigNum::SIGIOT as u32 - 1));
		const SIGBUS = (1 << (SigNum::SIGBUS as u32 - 1));
		const SIGFPE = (1 << (SigNum::SIGFPE as u32 - 1));
		const SIGKILL = (1 << (SigNum::SIGKILL as u32 - 1));
		const SIGUSR1 = (1 << (SigNum::SIGUSR1 as u32 - 1));
		const SIGSEGV = (1 << (SigNum::SIGSEGV as u32 - 1));
		const SIGUSR2 = (1 << (SigNum::SIGUSR2 as u32 - 1));
		const SIGPIPE = (1 << (SigNum::SIGPIPE as u32 - 1));
		const SIGALRM = (1 << (SigNum::SIGALRM as u32 - 1));
		const SIGTERM = (1 << (SigNum::SIGTERM as u32 - 1));
		const SIGSTKFLT = (1 << (SigNum::SIGSTKFLT as u32 - 1));
		const SIGCHLD = (1 << (SigNum::SIGCHLD as u32 - 1));
		const SIGCONT = (1 << (SigNum::SIGCONT as u32 - 1));
		const SIGSTOP = (1 << (SigNum::SIGSTOP as u32 - 1));
		const SIGTSTP = (1 << (SigNum::SIGTSTP as u32 - 1));
		const SIGTTIN = (1 << (SigNum::SIGTTIN as u32 - 1));
		const SIGTTOU = (1 << (SigNum::SIGTTOU as u32 - 1));
		const SIGURG = (1 << (SigNum::SIGURG as u32 - 1));
		const SIGXCPU = (1 << (SigNum::SIGXCPU as u32 - 1));
		const SIGXFSZ = (1 << (SigNum::SIGXFSZ as u32 - 1));
		const SIGVTALRM = (1 << (SigNum::SIGVTALRM as u32 - 1));
		const SIGPROF = (1 << (SigNum::SIGPROF as u32 - 1));
		const SIGWINCH = (1 << (SigNum::SIGWINCH as u32 - 1));
		const SIGIO = (1 << (SigNum::SIGIO as u32 - 1));
		const SIGPWR = (1 << (SigNum::SIGPWR as u32 - 1));
		const SIGSYS = (1 << (SigNum::SIGSYS as u32 - 1));
	}
}

impl From<SigNum> for SigFlag {
	fn from(value: SigNum) -> Self {
		SigFlag::from_bits_truncate(1 << (value as u32 - 1))
	}
}
