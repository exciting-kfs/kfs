use crate::file::read::sys_read;
use crate::file::write::sys_write;
use crate::interrupt::InterruptFrame;

pub mod errno;
use crate::pr_info;
use crate::process::exec::sys_exec;
use crate::process::{exit::sys_exit, fork::sys_fork, task::CURRENT};
use crate::signal::context::SigContext;
use crate::signal::handler::SigAction;
use crate::signal::{sys_sigaction, sys_signal, sys_sigreturn};

use self::errno::Errno;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let current = unsafe { CURRENT.get_mut() };

	let ret: Result<usize, Errno> = match frame.eax {
		1 => {
			pr_info!("PID[{}]: exited({})", current.get_pid(), frame.ebx);
			sys_exit(frame.ebx);
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
		42 => {
			pr_info!(
				"PID[{}]: DEBUG syscall called({})",
				current.get_pid(),
				frame.ebx as isize,
			);
			Ok(0)
		}
		11 => sys_exec(&mut frame, frame.ebx),
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
			sys_sigreturn(frame.ebx as *const SigContext)
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
