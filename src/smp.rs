use crate::interrupt::apic::local::LOCAL_APIC;

pub fn smp_id() -> usize {
	LOCAL_APIC.id()
}
