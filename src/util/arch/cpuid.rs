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

#[cfg(disable)]
mod cpuid_test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	fn cpuid_test() {
		assert_eq!(
			CPUID::run(0, 0),
			CPUID {
				eax: 13,
				ebx: 1752462657,
				ecx: 1145913699,
				edx: 1769238117
			}
		);
	}
}
