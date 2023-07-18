use crate::pr_info;
use crate::process::{exit::sys_exit, fork::sys_fork, task::CURRENT};

use super::InterruptFrame;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let current = unsafe { CURRENT.get_mut() };

	match frame.eax {
		1 => {
			pr_info!("PID[{}]: exited.", current.get_pid());
			sys_exit(frame.ebx);
		}
		2 => {
			sys_fork(&mut frame);
		}
		42 => {
			pr_info!(
				"PID[{}]: DEBUG syscall called ({})",
				current.get_pid(),
				frame.ebx
			);
		}
		_ => (),
	};
}
