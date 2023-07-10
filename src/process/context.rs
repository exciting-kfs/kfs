use crate::{
	interrupt::{irq_disable, irq_enable},
	pr_debug,
	smp::smp_id,
	sync::cpu_local::CpuLocal,
};

extern "C" {
	/// switch stack via exchange ESP
	/// see asm/interrupt.S
	pub fn switch_stack(prev_stack: *mut *mut usize, next_stack: *mut *mut usize);
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

		core::mem::replace(self, to)
	}
}

pub static IN_CONTEXT: CpuLocal<InContext> = CpuLocal::new(InContext::IrqDisabled);

pub fn context_switch(to: InContext) -> InContext {
	unsafe { IN_CONTEXT.get_mut().switch(to) }
}

pub fn cpu_context() -> InContext {
	unsafe { *IN_CONTEXT.get_mut() }
}
