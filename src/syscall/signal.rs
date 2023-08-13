use core::mem::{self, size_of};
use core::ptr::copy_nonoverlapping;

use crate::interrupt::InterruptFrame;
use crate::process::signal::sig_ctx::SigCtx;
use crate::process::signal::sig_flag::SigFlag;
use crate::process::signal::sig_handler::{SigAction, SigHandler};
use crate::process::signal::sig_info::SigInfo;
use crate::process::signal::sig_mask::SigMask;
use crate::process::signal::sig_num::SigNum;
use crate::{process::task::CURRENT, syscall::errno::Errno};

use super::*;

pub const SIG_ERR: usize = usize::MAX;
pub const SIG_DFL: usize = 0;
pub const SIG_IGN: usize = 1;

pub fn sys_signal(num: usize, handler: usize) -> Result<usize, Errno> {
	validate_user_addr(handler)?;
	let num = validate_sig_num(num)?;

	let new_handler = match handler {
		SIG_DFL => SigHandler::default(num),
		SIG_IGN => SigHandler::Ignore,
		_ => SigHandler::some(SigAction::new(handler, SigMask::empty(), SigFlag::DEFAULT)),
	};

	let mut table = unsafe { CURRENT.get_mut() }
		.user_ext_ok_or(Errno::UnknownErrno)? // kernel task.
		.signal
		.as_ref()
		.table
		.lock();
	let old_handler = mem::replace(&mut table[num.index()], new_handler);

	match old_handler {
		SigHandler::Some(sig_act) => Ok(sig_act.handler()),
		SigHandler::Ignore => Ok(SIG_IGN),
		_ => Ok(SIG_DFL),
	}
}

pub fn sys_sigaction(
	num: usize,
	act: *const SigAction,
	old: *mut SigAction,
) -> Result<usize, Errno> {
	validate_user_addr(act as usize)?;
	validate_user_addr(old as usize)?;
	let num = validate_sig_num(num)?;

	let mut table = unsafe { CURRENT.get_mut() }
		.user_ext_ok_or(Errno::UnknownErrno)? // kernel task.
		.signal
		.as_ref()
		.table
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
			SigHandler::Some(act) => act,
			_ => SigAction::empty(),
		};
		unsafe { *old = act };
	}

	Ok(0)
}

pub fn sys_sigreturn(frame: &InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	let sig_ctx = frame.ebx as *const SigCtx;
	let sig_info = frame.ecx as *const SigInfo;
	unsafe {
		// pr_debug!("sigreturn: {:p}, {}", sig_ctx, (*sig_ctx).intr);
		let current = CURRENT.get_mut();
		let signal = current.user_ext_ok_or(Errno::UnknownErrno)?.signal.as_ref();

		// restore the sig mask of the task.signal.
		signal.overwrite_mask((*sig_ctx).mask);

		// go to the next signal handler if the sig queue is not empty.
		signal.do_signal_repeat(frame);

		// else
		let syscall_ret = (*sig_ctx).syscall_ret;
		let flag = signal.get_handler(&(*sig_info).num).get_flag();
		*restart = is_syscall_restart(syscall_ret, flag);

		restore_interrupt_frame(&(*sig_ctx).intr_frame);
		restore_syscall_return((*sig_ctx).syscall_ret)
	}
}

unsafe fn restore_interrupt_frame(backup_frame: &InterruptFrame /* user stack */) {
	let current = CURRENT.get_mut();
	let frame = (current.kstack_base() - size_of::<InterruptFrame>()) as *mut InterruptFrame;
	copy_nonoverlapping(backup_frame as *const InterruptFrame, frame, 1);
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
	let memory = current
		.get_user_ext()
		.expect("must be user process")
		.lock_memory();

	let area = memory.get_vma().find_area(addr).ok_or(Errno::EFAULT)?;
	if !area.flags.contains(AreaFlag::Readable) {
		return Err(Errno::EFAULT);
	}
	Ok(())
}

#[inline(always)]
pub fn is_syscall_restart(syscall_ret: isize, flag: SigFlag) -> bool {
	syscall_ret == Errno::EINTR.as_ret() && flag.contains(SigFlag::Restart)
}
