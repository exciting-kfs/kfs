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
	/// - reentrance (x)
	NMI,
	/// `Hw irq`
	///
	/// - irq disabled
	/// - CpuLocal (o)
	/// - Singleton (o)
	/// - reentrance (x)
	HwIrq,
	/// `irq disabled`
	///
	/// - irq disabled
	/// - CpuLocal (o)
	/// - Singleton (o)
	/// - reentrance (x)
	IrqDisabled,
	/// `Kernel`
	///
	/// - irq enabled
	Kernel,
}

impl InContext {
	fn switch(&mut self, to: InContext) -> InContext {
		if *self == to {
			return to;
		}

		match to {
			Self::IrqDisabled => irq_disable(),
			Self::Kernel => irq_enable(),
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
