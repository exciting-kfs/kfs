use alloc::sync::Arc;

use crate::{
	process::{
		process_tree::PROCESS_TREE,
		relation::{session::Session, Pgid, Pid, Sid},
		task::{Task, CURRENT},
	},
	syscall::errno::Errno,
};

fn __set_pgid(task: &Arc<Task>, new_pgid: Pgid) -> Result<(), Errno> {
	let ext = task.user_ext_ok_or(Errno::EINVAL)?;
	let mut rel = ext.lock_relation();
	let pgrp = &mut rel.pgroup;
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
				Some(Session::new_pgroup(&sess, new_pgid))
			} else {
				None
			}
		});

	match new_pgrp {
		Some(g) => {
			rel.enter_new_pgroup(pid, g);
			Ok(())
		}
		None => Err(Errno::EPERM),
	}
}

pub fn sys_setpgid(pid: usize, pgid: usize) -> Result<usize, Errno> {
	let task = if pid == 0 {
		unsafe { CURRENT.get_mut().clone() }
	} else {
		PROCESS_TREE
			.get_task(Pid::from_raw(pid))
			.ok_or_else(|| Errno::ESRCH)?
	};

	let pgid = if pgid == 0 {
		Pgid::from(task.get_pid())
	} else {
		Pgid::from(Pid::from_raw(pgid))
	};

	__set_pgid(&task, pgid).map(|_| 0)
}

pub fn sys_setsid() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };
	let pid = current.get_pid();

	if current.get_sid() == Sid::from(pid) {
		return Err(Errno::EPERM);
	}

	let ext = current.user_ext_ok_or(Errno::EINVAL)?;
	ext.lock_relation().enter_new_session(pid);

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

pub fn sys_getsid(pid: usize) -> Result<usize, Errno> {
	let tree = PROCESS_TREE.lock();
	let current = unsafe { CURRENT.get_ref() };
	let task = match pid == 0 {
		true => Ok(current),
		false => tree.get(&Pid::from_raw(pid)).ok_or(Errno::ESRCH),
	}?;

	let sid = task.get_sid();
	if sid == current.get_sid() {
		Ok(sid.as_raw())
	} else {
		Err(Errno::EPERM)
	}
}

pub fn sys_getpgid(pid: usize) -> Result<usize, Errno> {
	let task = if pid == 0 {
		unsafe { CURRENT.get_ref() }.clone()
	} else {
		PROCESS_TREE
			.get_task(Pid::from_raw(pid))
			.ok_or_else(|| Errno::ESRCH)?
	};

	Ok(task.get_pgid().as_raw())
}

pub fn sys_getpgrp() -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };

	Ok(current.get_pgid().as_raw())
}
