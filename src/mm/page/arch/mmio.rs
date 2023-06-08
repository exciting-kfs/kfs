mod apic;

pub unsafe fn init() {
	apic::mapping_local_apic_registers().expect("mapping local apic");
	apic::mapping_io_apic_registers().expect("mapping io apic");
}
