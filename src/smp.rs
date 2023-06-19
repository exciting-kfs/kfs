pub use crate::interrupt::lapic_id;

pub fn smp_id() -> usize {
	lapic_id()
}
