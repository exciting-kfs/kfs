use core::mem;

use alloc::sync::Arc;

use super::task::{State, Task, CURRENT, TASK_QUEUE};
use crate::{
	interrupt::{irq_disable, irq_enable},
	pr_debug,
	smp::smp_id,
	sync::cpu_local::CpuLocal,
	x86::CPU_TASK_STATE,
};

extern "fastcall" {
	/// switch stack and call switch_task_finish
	///
	/// defined at asm/interrupt.S
	#[allow(improper_ctypes)]
	pub fn switch_stack(curr: *const Task, next: *const Task);
}

#[no_mangle]
pub unsafe extern "fastcall" fn switch_task_finish(curr: *const Task, next: *const Task) {
	let curr = Arc::from_raw(curr);
	let next = Arc::from_raw(next);

	let _ = mem::replace(CURRENT.get_mut(), next);

	CPU_TASK_STATE
		.get_mut()
		.change_kernel_stack(CURRENT.get_mut().kstack.base());

	if *curr.state.lock() != State::Exited {
		TASK_QUEUE.lock().push_back(curr);
	}

	context_switch(InContext::Kernel);
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum InContext {
	/// `NMI` and `CpuException`
	///
	/// - irq disabled
	/// - CpuLocal (?)
	/// - Singleton (x)
	/// - reentrance needed (x)
	NMI,
	/// `Hw irq`
	///
	/// - irq disabled
	/// - CpuLocal (o)
	/// - Singleton (o)
	/// - reentrance needed (x)
	HwIrq,
	/// `irq disabled`
	///
	/// - irq disabled
	/// - CpuLocal (o)
	/// - Singleton (o)
	/// - reentrance needed (x)
	IrqDisabled,
	/// `preempt disabled`
	///
	/// - irq enabled
	/// - CpuLocal (x)
	/// - Singleton (x)
	/// - reentrance needed (?)
	PreemptDisabled,
	/// `Kernel`
	///
	/// - irq enabled
	/// - CpuLocal (x)
	/// - Singleton (x)
	Kernel,
}

impl InContext {
	/// # CAUTION
	///
	/// If you want to switch from cpu local and singleton enabled context to disabled context,
	/// you must drop resources related with them before context switching.
	fn switch(&mut self, to: InContext) -> InContext {
		if *self == to {
			return to;
		}

		match to {
			Self::IrqDisabled => irq_disable(),
			Self::Kernel | Self::PreemptDisabled => irq_enable(),
			Self::NMI | Self::HwIrq => {}
		}

		pr_debug!(
			"CPU[{}]: context switched: from {:?} to {:?} ",
			smp_id(),
			self,
			to
		);

		mem::replace(self, to)
	}
}

pub static IN_CONTEXT: CpuLocal<InContext> = CpuLocal::new(InContext::IrqDisabled);

pub fn context_switch(to: InContext) -> InContext {
	unsafe { IN_CONTEXT.get_mut().switch(to) }
}

pub fn cpu_context() -> InContext {
	unsafe { *IN_CONTEXT.get_mut() }
}
