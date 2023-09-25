use crate::sync::CpuLocal;

static PREEMPT_COUNT: CpuLocal<usize> = CpuLocal::new(0);

#[derive(Debug)]
pub struct AtomicOps;

impl Drop for AtomicOps {
	fn drop(&mut self) {
		*unsafe { PREEMPT_COUNT.get_mut() } -= 1;
	}
}

pub fn preemptable() -> bool {
	*unsafe { PREEMPT_COUNT.get_mut() } == 0
}

pub fn get_preempt_count() -> usize {
	*unsafe { PREEMPT_COUNT.get_mut() }
}

pub fn preempt_disable() -> AtomicOps {
	*unsafe { PREEMPT_COUNT.get_mut() } += 1;
	AtomicOps
}
