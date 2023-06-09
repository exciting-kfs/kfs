mod apic;

pub unsafe fn init() {
	apic::init();
}
