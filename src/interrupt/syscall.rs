use kfs_macro::context;

use crate::file::read::sys_read;
use crate::file::write::sys_write;
use crate::interrupt::InterruptFrame;

pub mod errno;
use crate::pr_info;
use crate::process::context::yield_now;
use crate::process::family::TASK_TREE;
use crate::process::pid::Pid;
use crate::process::{exit::sys_exit, fork::sys_fork, task::CURRENT};

use self::errno::Errno;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let current = unsafe { CURRENT.get_mut() };

	let ret: Result<usize, Errno> = match frame.eax {
		1 => {
			pr_info!("PID[{}]: exited.", current.get_pid().as_raw());
			sys_exit(frame.ebx)
		}
		2 => sys_fork(&frame),
		3 => {
			pr_info!("syscall: read");
			sys_read(frame.ebx as isize, frame.ecx as *mut u8, frame.edx as isize)
		}
		4 => {
			pr_info!("syscall: write");
			sys_write(frame.ebx as isize, frame.ecx as *mut u8, frame.edx as isize)
		}
		7 => sys_waitpid(frame.ebx as isize, frame.ecx as *mut isize, frame.edx),
		42 => {
			pr_info!(
				"PID[{}]: DEBUG syscall called ({})",
				current.get_pid().as_raw(),
				frame.ebx
			);
			Ok(0)
		}
		_ => {
			pr_info!("syscall: the syscall {} is unsupported.", frame.eax);
			Ok(0)
		}
	};

	let ret = match ret {
		Ok(u) => u as isize,
		Err(e) => -(e as isize),
	};

	frame.eax = ret as usize;
}

#[context(irq_disabled)]
fn sys_waitpid(cpid: isize, _stat_loc: *mut isize, _option: usize) -> Result<usize, Errno> {
	let pid = unsafe { CURRENT.get_mut() }.get_pid();

	let cpid = if cpid > 0 {
		Some(Pid::from_raw(cpid as usize))
	} else {
		None
	};

	loop {
		let task_tree = unsafe { TASK_TREE.lock_manual() };
		let result = task_tree.try_wait_task(pid, cpid);
		unsafe { TASK_TREE.unlock_manual() };

		if let Ok((cpid, _)) = result {
			return Ok(cpid.as_raw());
		}

		yield_now();
	}
}
