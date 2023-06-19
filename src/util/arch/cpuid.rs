#[derive(Debug, PartialEq)]
pub struct CPUID {
	pub eax: usize,
	pub ebx: usize,
	pub ecx: usize,
	pub edx: usize,
}

impl CPUID {
	pub fn run(mut leaf: usize, mut sub_leaf: usize) -> Self {
		let ebx;
		let edx;

		unsafe {
			core::arch::asm!(
				"cpuid", inout("eax") leaf, out("ebx") ebx, inout("ecx") sub_leaf, out("edx") edx
			);
		}

		CPUID {
			eax: leaf,
			ebx,
			ecx: sub_leaf,
			edx,
		}
	}
}

mod cpuid_test {
	use crate::pr_info;

	use super::*;
	use kfs_macro::ktest;

	#[ktest(develop)]
	fn cpuid_test() {
		assert_eq!(
			CPUID::run(0, 0),
			CPUID {
				eax: 0x4,
				ebx: 0x756e6547,
				ecx: 0x6c65746e,
				edx: 0x49656e69
			}
		);

		pr_info!("{:x?}", CPUID::run(0, 0));
		pr_info!("{:x?}", CPUID::run(1, 0));
		pr_info!("{:x?}", CPUID::run(0x80000008, 0));
	}
}
