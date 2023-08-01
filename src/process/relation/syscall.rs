use alloc::sync::Arc;

use crate::{
	interrupt::syscall::errno::Errno,
	process::{
		process_tree::PROCESS_TREE,
		task::{Task, CURRENT},
	},
};

use super::{job::JobGroup, Pgid, Pid, Sid};

fn __set_pgid(task: &Arc<Task>, new_pgid: Pgid) -> Result<(), Errno> {
	let ext = task.user_ext(Errno::EINVAL)?;
	let mut relation = ext.lock_relation();
	let jgrp = &mut relation.jobgroup;
	let pgrp = &mut jgrp.pgroup;
	let sess = pgrp.sess.clone();
	let pid = task.get_pid();

	// already called `exec(2)`.
	if ext.was_exec_called() {
		return Err(Errno::EACCES);
	}

	if pgrp.get_pgid() == new_pgid {
		return Ok(());
	}

	let sess_lock = sess.lock();
	let new_pgrp = sess_lock
		.get(&new_pgid)
		.and_then(|w| w.upgrade())
		.or_else(|| {
			drop(sess_lock);
			if pid.as_raw() == new_pgid.as_raw() {
				Some(JobGroup::new_pgroup_in_session(new_pgid, &sess))
			} else {
				None
			}
		});

	match new_pgrp {
		Some(g) => {
			jgrp.enter_new_pgroup(pid, g);
			Ok(())
		}
		None => Err(Errno::EPERM),
	}
}

pub fn sys_setpgid(pid: usize, pgid: usize) -> Result<usize, Errno> {
	let task = if pid == 0 {
		unsafe { CURRENT.get_mut().clone() }
	} else {
		let ptree = PROCESS_TREE.lock();

		ptree
			.get(&Pid::from_raw(pid))
			.ok_or_else(|| Errno::ESRCH)?
			.clone()
	};

	let pgid = if pgid == 0 {
		Pgid::from(task.get_pid())
	} else {
		Pgid::from_raw(pgid)
	};

	__set_pgid(&task, pgid).map(|_| 0)
}

pub fn sys_getpgrp() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_pgid().as_raw())
}

pub fn sys_setsid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };
	let pid = current.get_pid();
	let ext = current.user_ext(Errno::EINVAL)?;
	let mut rel = ext.lock_relation();
	let jgrp = &mut rel.jobgroup;

	if jgrp.get_sid() == Sid::from(pid) {
		return Err(Errno::EPERM);
	}

	jgrp.enter_new_session(pid);

	Ok(pid.as_raw())
}

pub fn sys_getpid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_pid().as_raw())
}

pub fn sys_getppid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_ppid().as_raw())
}

pub fn sys_getpgid(pid: usize) -> Result<usize, Errno> {
	let task = PROCESS_TREE
		.get_task(Pid::from_raw(pid))
		.ok_or_else(|| Errno::ESRCH)?;

	Ok(task.get_pgid().as_raw())
}

pub fn sys_getsid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_sid().as_raw())
}
