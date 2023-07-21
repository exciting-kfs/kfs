use super::{sig_flag::SigFlag, sig_num::SigNum};

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
pub struct SigAction {
	pub addr: usize,
	pub mask: SigFlag,
}

impl SigAction {
	pub const fn empty() -> Self {
		Self {
			addr: 0,
			mask: SigFlag::empty(),
		}
	}

	pub fn new(addr: usize, mask: SigFlag) -> Self {
		SigAction { addr, mask }
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
			SIGABRT => Core,
			SIGALRM => Terminate,
			SIGBUS => Core,
			SIGCHLD => Ignore,
			SIGCONT => Continue,
			SIGFPE => Core,
			SIGHUP => Terminate,
			SIGILL => Core,
			SIGINT => Terminate,
			SIGIO => Terminate,
			SIGIOT => Core,
			SIGKILL => Terminate,
			SIGPIPE => Terminate,
			SIGPROF => Terminate,
			SIGPWR => Terminate,
			SIGQUIT => Core,
			SIGSEGV => Core,
			SIGSTKFLT => Terminate,
			SIGSTOP => Stop,
			SIGTSTP => Stop,
			SIGSYS => Core,
			SIGTERM => Terminate,
			SIGTRAP => Core,
			SIGTTIN => Stop,
			SIGTTOU => Stop,
			SIGURG => Ignore,
			SIGUSR1 => Terminate,
			SIGUSR2 => Terminate,
			SIGVTALRM => Terminate,
			SIGXCPU => Core,
			SIGXFSZ => Core,
			SIGWINCH => Ignore,
			Unknown => unreachable!(),
		}
	}
}
