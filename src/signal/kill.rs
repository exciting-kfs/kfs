use alloc::sync::Arc;

use crate::interrupt::syscall::errno::Errno;
use crate::process::relation::job::group::ProcessGroup;
use crate::process::relation::job::SESSION_TREE;
use crate::process::relation::{Pgid, Pid};
use crate::process::task::{Task, CURRENT, PROCESS_TREE};
use crate::signal::sig_code::SigCode;
use crate::signal::sig_info::SigInfo;
use crate::sync::locked::Locked;

use super::send_signal_to;
use super::sig_num::SigNum;

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
	let sess_tree = SESSION_TREE.lock();
	let session = sess_tree.get(&current.get_sid()).unwrap();

	let sess_lock = session.lock();
	let pgroup = sess_lock.get(&pgid).ok_or_else(|| Errno::ESRCH)?;

	kill_pgroup(pgroup, siginfo)
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

fn kill_pgroup(pgroup: &Arc<Locked<ProcessGroup>>, siginfo: &SigInfo) -> Result<(), Errno> {
	let pgroup_lock = pgroup.lock();

	for pid in pgroup_lock.members() {
		if let Some(task) = PROCESS_TREE.lock().get(pid) {
			if task.get_pid().as_raw() != siginfo.pid {
				let _ = kill_process(task, siginfo);
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
