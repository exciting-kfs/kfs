use alloc::sync::Arc;

use crate::process::process_tree::PROCESS_TREE;
use crate::process::relation::pgroup::ProcessGroup;
use crate::process::relation::{Pgid, Pid};
use crate::process::signal::send_signal_to;
use crate::process::signal::sig_code::SigCode;
use crate::process::signal::sig_info::SigInfo;
use crate::process::signal::sig_num::SigNum;
use crate::process::task::{Task, CURRENT};
use crate::syscall::errno::Errno;

pub fn sys_kill(pid: isize, sig: isize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	let num = SigNum::from_usize(sig as usize).ok_or_else(|| Errno::EINVAL)?;

	let siginfo = SigInfo {
		num,
		pid: current.get_pid().as_raw(),
		uid: current.get_uid(),
		code: SigCode::SI_USER,
	};

	match pid {
		-1 => kill_everyone(&siginfo),
		0 => kill_pgid(current.get_pgid(), &siginfo),
		x if x < 0 => kill_pgid(Pgid::from_raw(-x as usize), &siginfo),
		x if x > 0 => kill_pid(Pid::from_raw(x as usize), &siginfo),
		_ => unreachable!("bug"),
	}
	.map(|_| 0)
}

fn kill_pid(pid: Pid, siginfo: &SigInfo) -> Result<(), Errno> {
	let ptree = PROCESS_TREE.lock();
	let task = ptree.get(&pid).ok_or_else(|| Errno::ESRCH)?;

	kill_process(task, siginfo)
}

fn kill_pgid(pgid: Pgid, siginfo: &SigInfo) -> Result<(), Errno> {
	let current = unsafe { CURRENT.get_mut() };
	let rel = current.user_ext_ok_or(Errno::EPERM)?.lock_relation();
	let pgroup = rel
		.get_session()
		.lock()
		.get(&pgid)
		.and_then(|w| w.upgrade())
		.ok_or(Errno::ESRCH)?;

	kill_pgroup(&pgroup, siginfo)
}

fn kill_process(target: &Arc<Task>, siginfo: &SigInfo) -> Result<(), Errno> {
	// init process cannot receive signal
	if target.get_pid().as_raw() == 1 {
		return Err(Errno::EPERM);
	}

	if siginfo.uid != 0 && siginfo.uid != target.get_uid() {
		return Err(Errno::EPERM);
	}

	send_signal_to(target, siginfo)
}

fn kill_pgroup(pgroup: &Arc<ProcessGroup>, siginfo: &SigInfo) -> Result<(), Errno> {
	for (_, weak) in pgroup.lock_members().iter() {
		if let Some(task) = weak.upgrade() {
			if task.get_pid().as_raw() != siginfo.pid {
				let _ = kill_process(&task, siginfo);
			}
		}
	}

	Ok(())
}

fn kill_everyone(siginfo: &SigInfo) -> Result<(), Errno> {
	for (_, task) in PROCESS_TREE.lock().members() {
		if task.get_pid().as_raw() != siginfo.pid {
			let _ = kill_process(task, siginfo);
		}
	}

	Ok(())
}
