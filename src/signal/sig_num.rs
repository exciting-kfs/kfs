use core::mem::transmute;

#[repr(usize)]
#[derive(Clone, Copy, Debug)]
pub enum SigNum {
	Unknown,
	SIGHUP,
	SIGINT,
	SIGQUIT,
	SIGILL,
	SIGTRAP,
	SIGABRT,
	SIGIOT,
	SIGBUS,
	SIGFPE,
	SIGKILL,
	SIGUSR1,
	SIGSEGV,
	SIGUSR2,
	SIGPIPE,
	SIGALRM,
	SIGTERM,
	SIGSTKFLT,
	SIGCHLD,
	SIGCONT,
	SIGSTOP,
	SIGTSTP,
	SIGTTIN,
	SIGTTOU,
	SIGURG,
	SIGXCPU,
	SIGXFSZ,
	SIGVTALRM,
	SIGPROF,
	SIGWINCH,
	SIGIO,
	SIGPWR,
	SIGSYS,
}

impl SigNum {
	pub fn from_usize(num: usize) -> Option<Self> {
		use SigNum::*;
		if SIGHUP as usize <= num && num <= SIGSYS as usize {
			Some(unsafe { transmute(num) })
		} else {
			None
		}
	}
}
