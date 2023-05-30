use core::arch::asm;

use crate::sync::singleton::Singleton;

pub static MSR_APIC_BASE: Singleton<Msr> = Singleton::new(Msr::new(0x1b));

pub struct Msr {
	ptr: usize,
}

impl Msr {
	pub const fn new(ptr: usize) -> Self {
		Msr { ptr }
	}

	/// # Safety
	///
	/// - privilege level = 0
	pub fn write(&self, val: MsrVal) {
		unsafe {
			asm!("wrmsr", in("eax") val.low, in("ecx") self.ptr, in("edx") val.high);
		}
	}

	/// # Safety
	///
	/// - privilege level = 0
	pub fn read(&self) -> MsrVal {
		let mut high;
		let mut low;

		unsafe {
			asm!("rdmsr", out("eax") low, in("ecx") self.ptr, out("edx") high);
		}

		MsrVal::new(high, low)
	}
}

#[derive(Debug)]
pub struct MsrVal {
	pub high: usize,
	pub low: usize,
}

impl MsrVal {
	pub fn new(high: usize, low: usize) -> Self {
		MsrVal { high, low }
	}
}

mod msr_test {
	use crate::pr_info;

	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	fn test() {
		let a = Msr::new(0x1b);
		let def = Msr::new(0x2ff); // MTRR_DEF_TYPE
		pr_info!("apic_base: {:x?}", a.read());
		pr_info!("mtrr_def_type: {:x?}", def.read());

		// MTRR_PHYSBASE(0 ~ 9: 200H.step(2)), MTRR_PHYSMASK(0 ~ 9: 201H.step(2))
		for i in 0..10 {
			let base = Msr::new(0x200 + i * 2);
			let mask = Msr::new(0x201 + i * 2);

			pr_info!("{}: {:x?} : {:x?}", i, base.read(), mask.read());
		}
	}
}
