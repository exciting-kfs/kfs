mod apic;

pub unsafe fn init() {
	apic::mapping_apic_registers().expect("mapping apic");
}
