pub mod errno;
pub mod exec;
pub mod fork;
pub mod kill;
pub mod relation;
pub mod signal;
pub mod wait;

use core::mem::transmute;

use alloc::sync::Arc;

use crate::driver::pipe::sys_pipe;
use crate::driver::tty;
use crate::file::close::sys_close;
use crate::file::read::sys_read;
use crate::file::write::sys_write;
use crate::file::{File, OpenFlag};
use crate::interrupt::InterruptFrame;

use crate::mm::user::mmap::sys_mmap;
use crate::pr_info;
use crate::process::exit::sys_exit;
use crate::process::signal::sig_handler::SigAction;
use crate::process::task::CURRENT;
use crate::process::uid::{sys_getuid, sys_setuid};
use crate::scheduler::sys_sched_yield;

use self::errno::Errno;
use self::exec::sys_exec;
use self::fork::sys_fork;
use self::kill::sys_kill;
use self::relation::{
	sys_getpgid, sys_getpgrp, sys_getpid, sys_getppid, sys_getsid, sys_setpgid, sys_setsid,
};
use self::signal::{sys_sigaction, sys_signal, sys_sigreturn};
use self::wait::sys_waitpid;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let mut restart = true;
	let mut ret = Err(Errno::UnknownErrno);
	let signal = unsafe {
		CURRENT
			.get_mut()
			.get_user_ext()
			.expect("user task")
			.signal
			.as_ref()
	};

	while restart {
		restart = false;
		ret = syscall(&mut frame, &mut restart);

		if let Some(_) = signal.do_signal(&frame, syscall_return_to_isize(&ret)) {
			restart = true;
		}
		// use crate::pr_debug;
		// pr_debug!("syscall: ret: {:?}", ret);
		// pr_debug!("syscall: restart: {}", restart);
	}

	// Because of signal system, This can be `return` from timer interrupt.
	// To preserve previous `eax` value, We should check where this `return` go.
	if frame.handler == handle_syscall_impl as usize {
		frame.eax = syscall_return_to_isize(&ret) as usize;
	}
}

fn syscall(frame: &mut InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	// let current = unsafe { CURRENT.get_mut() };
	match frame.eax {
		1 => {
			// pr_info!("PID[{}]: exited({})", current.get_pid().as_raw(), frame.ebx);
			sys_exit(frame.ebx);
		}
		2 => sys_fork(frame),
		3 => {
			// pr_debug!("syscall: read");
			sys_read(frame.ebx as isize, frame.ecx as *mut u8, frame.edx)
		}
		4 => {
			// pr_debug!("syscall: write");
			sys_write(frame.ebx as isize, frame.ecx as *mut u8, frame.edx)
		}
		5 => sys_open(),
		6 => sys_close(frame.ebx as isize),
		7 => sys_waitpid(frame.ebx as isize, frame.ecx as *mut isize, frame.edx),
		11 => sys_exec(frame, frame.ebx),
		20 => sys_getpid(),
		37 => sys_kill(frame.ebx as isize, frame.ecx as isize),
		42 => sys_pipe(frame.ebx),
		48 => {
			pr_info!("syscall: signal: {}, {:x}", frame.ebx, frame.ecx);
			sys_signal(frame.ebx, frame.ecx)
		}
		57 => sys_setpgid(frame.ebx, frame.ecx),
		64 => sys_getppid(),
		65 => sys_getpgrp(),
		66 => sys_setsid(),
		67 => {
			// pr_info!(
			// 	"syscall: sigaction: {}, {:x}, {:x}",
			// 	frame.ebx,
			// 	frame.ecx,
			// 	frame.edx
			// );
			sys_sigaction(
				frame.ebx,
				frame.ecx as *const SigAction,
				frame.edx as *mut SigAction,
			)
		}
		119 => {
			// pr_info!("syscall: sigreturn: {:p}", &frame);
			sys_sigreturn(frame, restart)
		}
		132 => sys_getpgid(frame.ebx),
		147 => sys_getsid(),
		158 => sys_sched_yield(),
		192 => sys_mmap(
			frame.ebx,
			frame.ecx,
			frame.edx as i32,
			frame.esi as i32,
			frame.edi as i32,
			frame.ebp as isize,
		)
		.map_err(|_| Errno::UnknownErrno), // FIXME: proper return type
		199 => sys_getuid(),
		213 => sys_setuid(frame.ebx),
		_ => {
			pr_info!("syscall: the syscall {} is unsupported.", frame.eax);
			Ok(0)
		}
	}
}

pub fn syscall_return_to_isize(result: &Result<usize, Errno>) -> isize {
	match result {
		Ok(u) => *u as isize,
		Err(e) => e.as_ret(),
	}
}

pub fn restore_syscall_return(result: isize) -> Result<usize, Errno> {
	if result < 0 {
		Err(unsafe { transmute(-result) })
	} else {
		Ok(result as usize)
	}
}

// FIXME Rough implementation for test.
pub fn sys_open() -> Result<usize, Errno> {
	let ext = unsafe { CURRENT.get_mut() }.user_ext_ok_or(Errno::EPERM)?;
	let tty = tty::alloc().ok_or(Errno::UnknownErrno)?;
	let sess = &ext.lock_relation().get_session();

	// TODO Atomic
	tty.lock().connect(Arc::downgrade(sess));
	sess.lock().set_ctty(tty.clone());

	let file = Arc::new(File::new(tty, OpenFlag::O_RDWR));
	let mut fd_table = ext.lock_fd_table();
	let _ = fd_table.alloc_fd(file.clone()).ok_or(Errno::ENFILE)?;
	let fd = fd_table.alloc_fd(file).ok_or(Errno::ENFILE)?;

	Ok(fd.index())
}
