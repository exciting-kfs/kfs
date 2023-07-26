pub mod context;
pub mod handler;
pub mod sig_code;
pub mod sig_flag;
pub mod sig_mask;
pub mod sig_num;

use core::{
	array,
	mem::{self, size_of, variant_count},
	ptr::{self, addr_of, copy_nonoverlapping},
};

use alloc::collections::LinkedList;
use kfs_macro::context;

use crate::{
	config::TRAMPOLINE_BASE,
	interrupt::{syscall::errno::Errno, InterruptFrame},
	pr_debug,
	process::task::CURRENT,
	signal::{handler::SigHandler, sig_flag::SigFlag, sig_num::SigNum},
	sync::locked::Locked,
};

use self::{context::SigContext, handler::SigAction, sig_code::SigCode, sig_mask::SigMask};

extern "C" {
	fn signal_trampoline();
	fn __trampoline_start();
	fn go_to_signal_handler(intr_frame: *const InterruptFrame, new_esp: usize, handler: usize)
		-> !;
}

pub fn trampoline_address(addr: usize) -> usize {
	TRAMPOLINE_BASE + (addr - __trampoline_start as usize)
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SigInfo {
	pub num: SigNum,   /* Signal number */
	pub pid: usize,    /* Sending process ID */
	pub uid: usize,    /* Real user ID of sending process */
	pub code: SigCode, /* Signal code: why this signal was sent. */
}

// struct sig_info {
// 	...
// int      errno;        /* An errno value */
// int      trapno        /* Trap number that caused hardware-generated signal (unused on most architectures) */
// int      si_status;    /* Exit value or signal */
// clock_t  si_utime;     /* User time consumed */
// clock_t  si_stime;     /* System time consumed */
// union sigval si_value; /* Signal value */
// int      si_int;       /* POSIX.1b signal */
// void    *si_ptr;       /* POSIX.1b signal */
// int      si_overrun;   /* Timer overrun count;  POSIX.1b timers */
// int      si_timerid;   /* Timer ID; POSIX.1b timers */
// void    *si_addr;      /* Memory location which caused fault */
// long     si_band;      /* Band event (was int in glibc 2.3.2 and earlier) */
// int      si_fd;        /* File descriptor */
// short    si_addr_lsb;  /* Least significant bit of address (since Linux 2.6.32) */
// void    *si_lower;     /* Lower bound when address violation occurred (since Linux 3.19) */
// void    *si_upper;     /* Upper bound when address violation occurred (since Linux 3.19) */
// int      si_pkey;      /* Protection key on PTE that caused fault (since Linux 4.6) */
// void    *si_call_addr; /* Address of system call instruction (since Linux 3.5) */
// int      si_syscall;   /* Number of attempted system call (since Linux 3.5) */
// unsigned int si_arch;  /* Architecture of attempted system call
//}

pub const SIG_ERR: usize = usize::MAX;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

#[context(irq_disabled)]
pub fn sys_signal(num: usize, handler: usize) -> Result<usize, Errno> {
	validate_user_addr(handler)?;
	let num = validate_sig_num(num)?;

	let new_handler = match handler {
		h if h == SIG_DFL => SigHandler::default(num),
		h if h == SIG_IGN => SigHandler::Ignore,
		_ => SigHandler::some(SigAction::new(handler, SigMask::empty(), SigFlag::empty())),
	};

	let mut table = unsafe { CURRENT.get_mut() }
		.signal
		.as_ref()
		.ok_or(Errno::UnknownErrno)? // kernel task.
		.sig_table
		.lock();
	let old_handler = mem::replace(&mut table[num.index()], new_handler);

	match old_handler {
		SigHandler::Some(sig_act) => Ok(sig_act.handler()),
		SigHandler::Ignore => Ok(SIG_IGN),
		_ => Ok(SIG_DFL),
	}
}

#[context(irq_disabled)]
pub fn sys_sigaction(
	num: usize,
	act: *const SigAction,
	old: *mut SigAction,
) -> Result<usize, Errno> {
	validate_user_addr(act as usize)?;
	validate_user_addr(old as usize)?;
	let num = validate_sig_num(num)?;

	let mut table = unsafe { CURRENT.get_mut() }
		.signal
		.as_ref()
		.ok_or(Errno::UnknownErrno)? // kernel task.
		.sig_table
		.lock();

	let old_handler = if act.is_null() {
		table[num.index()].clone()
	} else {
		let act = unsafe { &*act };
		let new_handler = match act.flag().contains(SigFlag::ResetHand) {
			true => SigHandler::default(num), // SIGTRAP SIGILL ?
			false => SigHandler::some(act.clone()),
		};
		mem::replace(&mut table[num.index()], new_handler)
	};

	if !old.is_null() {
		let act = match old_handler {
			SigHandler::Some(sig_act) => sig_act,
			_ => SigAction::empty(),
		};
		unsafe { *old = act };
	}

	Ok(0)
}

pub fn sys_sigreturn(sig_ctx: *const SigContext /* user stack */) -> Result<usize, Errno> {
	unsafe {
		// pr_debug!("sigreturn: {:p}, {}", sig_ctx, (*sig_ctx).intr);
		let current = CURRENT.get_mut();
		let frame = (current.kstack_base() - size_of::<InterruptFrame>()) as *mut InterruptFrame;
		copy_nonoverlapping(addr_of!((*sig_ctx).intr), frame, 1);
		current
			.signal
			.as_ref()
			.ok_or(Errno::UnknownErrno)?
			.set_signal_mask((*sig_ctx).mask);
		current.do_signal();
		Ok(0)
	}
}

fn validate_sig_num(num: usize) -> Result<SigNum, Errno> {
	let num = SigNum::from_usize(num).ok_or(Errno::EINVAL)?;
	if let SigNum::KILL | SigNum::STOP = num {
		return Err(Errno::EINVAL);
	}
	Ok(num)
}

fn validate_user_addr(addr: usize) -> Result<(), Errno> {
	use crate::mm::user::vma::AreaFlag;

	if addr == SIG_DFL || addr == SIG_IGN {
		return Ok(());
	}

	let current = unsafe { CURRENT.get_mut() };
	let memory = current.lock_memory().unwrap();

	let area = memory.get_vma().find_area(addr).ok_or(Errno::EFAULT)?;
	if !area.flags.contains(AreaFlag::Readable) {
		return Err(Errno::EFAULT);
	}
	Ok(())
}

pub struct Signal {
	sig_queue: Locked<LinkedList<SigInfo>>,
	sig_mask: Locked<SigMask>,
	pub sig_table: Locked<[SigHandler; variant_count::<SigNum>()]>,
}

impl Signal {
	pub fn new() -> Self {
		Self {
			sig_mask: Locked::new(SigMask::empty()),
			sig_queue: Locked::new(LinkedList::new()),
			sig_table: Locked::new(array::from_fn(|i| {
				SigHandler::default(SigNum::from_usize(i + 1).unwrap())
			})),
		}
	}

	#[context(irq_disabled)]
	pub fn set_signal_mask(&self, mask: SigMask) {
		let _lock = self.sig_mask.lock();
		unsafe { *self.sig_mask.as_mut_ptr() = mask }
	}

	pub fn clone_for_fork(&self) -> Self {
		Self {
			sig_queue: Locked::new(LinkedList::new()),
			sig_mask: self.sig_mask.clone(),
			sig_table: self.sig_table.clone(),
		}
	}

	#[context(irq_disabled)]
	fn calc_mask(&self, act: &SigAction, info: &SigInfo) -> SigMask {
		let lock = self.sig_mask.lock();
		let o_mask = *lock;
		let n_mask = if act.flag().contains(SigFlag::NoDefer) {
			o_mask - info.num.into() | act.mask()
		} else {
			o_mask | info.num.into() | act.mask()
		};
		unsafe { ptr::replace(self.sig_mask.as_mut_ptr(), n_mask) }
	}

	#[context(irq_disabled)]
	pub fn recv_signal(&self, sig_info: SigInfo) {
		let sig_mask = self.sig_mask.lock();
		if sig_mask.contains(sig_info.num.into()) {
			return;
		}

		pr_debug!("received [{:?}] from pid[{}]", sig_info.num, sig_info.pid);
		let mut sig_queue = self.sig_queue.lock();

		match sig_info.num {
			SigNum::KILL | SigNum::STOP => sig_queue.push_front(sig_info),
			_ => sig_queue.push_back(sig_info),
		}
	}

	pub fn do_signal(&self, intr_frame: *const InterruptFrame) -> Option<()> {
		let sig_info = self.get_event()?;
		let sig_handler = self.get_handler_info(&sig_info);
		match sig_handler {
			SigHandler::Some(act) => unsafe { self.sig_action(act, sig_info, intr_frame) },
			SigHandler::Ignore => Some(()), // TODO core, term, stop, cont
			SigHandler::Terminate | SigHandler::Core => {
				pr_debug!("sig term!");
				Some(())
			}
			SigHandler::Continue => {
				pr_debug!("sig cont!");
				Some(())
			}
			SigHandler::Stop => {
				pr_debug!("sig stop!");
				Some(())
			}
		}
	}

	/// # Safety
	///
	/// Must clean up lock and global variable before this function call.
	unsafe fn sig_action(
		&self,
		act: SigAction,
		info: SigInfo,
		intr_frame: *const InterruptFrame,
	) -> ! {
		let o_mask = self.calc_mask(&act, &info);
		let sig_ctx = SigContext::new(intr_frame, o_mask);

		let mut esp = (*intr_frame).esp;
		push_to_user_stack(
			&mut esp,
			&sig_ctx as *const SigContext as *const u8,
			size_of::<SigContext>(),
		);
		push_to_user_stack(
			&mut esp,
			&info as *const SigInfo as *const u8,
			size_of::<SigInfo>(),
		);

		let mut func_frame = [0; 4];
		make_frame(&mut func_frame, esp, info);
		push_to_user_stack(
			&mut esp,
			func_frame.as_ptr().cast(),
			func_frame.len() * size_of::<usize>(),
		);
		pr_debug!("go_to_signal_handler");
		go_to_signal_handler(intr_frame, esp, act.handler());
	}

	#[context(irq_disabled)]
	fn get_event(&self) -> Option<SigInfo> {
		let mut queue = self.sig_queue.lock();
		let mask = self.sig_mask.lock();

		let mut ret = None;
		let mut not = LinkedList::new();
		while let Some(info) = queue.pop_front() {
			if mask.contains(info.num.into()) {
				not.push_back(info)
			} else {
				ret = Some(info);
				break;
			}
		}
		queue.extend(not);

		ret
	}

	#[context(irq_disabled)]
	fn get_handler_info<'a>(&self, sig_info: &SigInfo) -> SigHandler {
		let table = self.sig_table.lock();
		let handler = &table[sig_info.num.index()];
		handler.clone()
	}
}

/// push data to user stack.
unsafe fn push_to_user_stack(esp: &mut usize, src: *const u8, len: usize) {
	*esp -= len;
	copy_nonoverlapping(src, (*esp) as *mut _, len);
}

fn make_frame(frame: &mut [usize], user_esp: usize, sig_info: SigInfo) {
	let trampoline = trampoline_address(signal_trampoline as usize);

	frame[0] = trampoline;
	frame[1] = sig_info.num as usize;
	frame[2] = user_esp;
	frame[3] = user_esp + size_of::<SigInfo>()
}
