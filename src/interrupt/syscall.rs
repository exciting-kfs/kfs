use core::mem::transmute;

use crate::file::read::sys_read;
use crate::file::write::sys_write;
use crate::interrupt::InterruptFrame;

pub mod errno;
use crate::pr_info;
use crate::process::exec::sys_exec;
use crate::process::{exit::sys_exit, fork::sys_fork, task::CURRENT};
use crate::signal::sig_handler::SigAction;
use crate::signal::{sys_sigaction, sys_signal, sys_sigreturn};

use self::errno::Errno;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let mut restart = true;
	let mut ret = Err(Errno::UnknownErrno);
	let signal = unsafe { CURRENT.get_mut().signal.as_ref().expect("user task") };

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

	frame.eax = syscall_return_to_isize(&ret) as usize;
}

fn syscall(frame: &mut InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_mut() };
	match frame.eax {
		1 => {
			pr_info!("PID[{}]: exited({})", current.get_pid(), frame.ebx);
			sys_exit(frame.ebx);
		}
		2 => sys_fork(frame),
		3 => {
			pr_info!("syscall: read");
			sys_read(frame.ebx as isize, frame.ecx as *mut u8, frame.edx as isize)
		}
		4 => {
			pr_info!("syscall: write");
			sys_write(frame.ebx as isize, frame.ecx as *mut u8, frame.edx as isize)
		}
		42 => {
			pr_info!(
				"PID[{}]: DEBUG syscall called({})",
				current.get_pid(),
				frame.ebx as isize,
			);
			Ok(0)
		}
		11 => sys_exec(frame, frame.ebx),
		48 => {
			pr_info!("syscall: signal: {}, {:x}", frame.ebx, frame.ecx);
			sys_signal(frame.ebx, frame.ecx)
		}
		67 => {
			pr_info!(
				"syscall: sigaction: {}, {:x}, {:x}",
				frame.ebx,
				frame.ecx,
				frame.edx
			);
			sys_sigaction(
				frame.ebx,
				frame.ecx as *const SigAction,
				frame.edx as *mut SigAction,
			)
		}
		119 => {
			pr_info!("syscall: sigreturn: {:p}", &frame);
			sys_sigreturn(frame, restart)
		}
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
