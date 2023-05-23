use super::{
	addr_to_pfn,
	constant::{PAGE_SHIFT, VM_OFFSET},
	pfn_to_addr,
};

#[inline]
pub const fn phys_to_virt(addr: usize) -> usize {
	addr.wrapping_add(VM_OFFSET)
}

#[inline]
pub const fn virt_to_phys(addr: usize) -> usize {
	addr.wrapping_sub(VM_OFFSET)
}

#[inline]
pub const fn pfn_virt_to_phys(pfn: usize) -> usize {
	addr_to_pfn(virt_to_phys(pfn_to_addr(pfn)))
}

#[inline]
pub const fn pfn_phys_to_virt(pfn: usize) -> usize {
	addr_to_pfn(phys_to_virt(pfn_to_addr(pfn)))
}

#[inline]
pub const fn prev_align(p: usize, align: usize) -> usize {
	(p - 1) & !(align - 1)
}

#[inline]
pub const fn next_align(p: usize, align: usize) -> usize {
	(p + align - 1) & !(align - 1)
}

#[inline]
pub const fn next_align_64(p: u64, align: u64) -> u64 {
	(p + align - 1) & !(align - 1)
}

#[inline]
pub const fn is_aligned(addr: usize, align: usize) -> bool {
	addr % align == 0
}

#[inline]
pub const fn is_aligned_64(addr: u64, align: u64) -> bool {
	addr % align == 0
}

#[inline]
pub const fn align_of_rank(rank: usize) -> usize {
	1 << PAGE_SHIFT << rank
}

#[inline]
pub const fn size_of_rank(rank: usize) -> usize {
	1 << PAGE_SHIFT << rank
}

/// It is wrapper function for `bsf` x86 instruction.
///
/// # Safety
/// `data` must not be 0. It is undefined behavior for x86 cpu.
pub unsafe fn bit_scan_forward(data: usize) -> usize {
	let ret;
	unsafe {
		core::arch::asm!(
			"bsf {0}, {1}",
			out(reg) ret,
			in(reg) data
		);
	}
	ret
}

/// It is wrapper function for `bsr` x86 instruction.
///
/// # Safety
/// `data` must not be 0. It is undefined behavior for x86 cpu.
pub unsafe fn bit_scan_reverse(data: usize) -> usize {
	let ret;
	unsafe {
		core::arch::asm!(
			"bsr {0}, {1}",
			out(reg) ret,
			in(reg) data
		);
	}
	ret
}

mod test {
	use super::*;
	use kfs_macro::ktest;

	#[ktest]
	fn test_bsfr() {
		unsafe {
			let ret = bit_scan_forward(0x01);
			assert_eq!(ret, 0);
			let ret = bit_scan_forward(0x100);
			assert_eq!(ret, 8);
			let ret = bit_scan_forward(0x0101);
			assert_eq!(ret, 0);

			let ret = bit_scan_reverse(0x01);
			assert_eq!(ret, 0);
			let ret = bit_scan_reverse(0x100);
			assert_eq!(ret, 8);
			let ret = bit_scan_reverse(0x0101);
			assert_eq!(ret, 8);
		}
	}
}
