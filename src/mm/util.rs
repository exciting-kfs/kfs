use super::constant::*;
use core::{alloc::Layout, arch::asm};

#[inline]
pub const fn addr_to_pfn(addr: usize) -> usize {
	addr >> PAGE_SHIFT
}

#[inline]
pub const fn addr_to_pfn_64(addr: u64) -> u64 {
	addr >> PAGE_SHIFT
}

#[inline]
pub const fn pfn_to_addr(pfn: usize) -> usize {
	pfn << PAGE_SHIFT
}

#[inline]
pub const fn rank_to_pages(rank: usize) -> usize {
	1 << rank
}

#[inline]
pub const fn rank_to_size(rank: usize) -> usize {
	rank_to_pages(rank) * PAGE_SIZE
}

pub const fn size_to_rank(size: usize) -> usize {
	match size {
		0 => 0,
		x => 32 - ((x - 1) >> PAGE_SHIFT).leading_zeros() as usize,
	}
}

#[inline]
pub const fn phys_to_virt(paddr: usize) -> usize {
	paddr.wrapping_add(VM_OFFSET)
}

#[inline]
pub const fn virt_to_phys(vaddr: usize) -> usize {
	vaddr.wrapping_sub(VM_OFFSET)
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

#[inline]
pub fn invlpg(vaddr: usize) {
	unsafe { asm!("invlpg [{vaddr}]", vaddr = in(reg) vaddr, options(nostack, preserves_flags)) };
}

/// It is wrapper function for `bsf` x86 instruction.
///
/// # Safety
/// `data` must not be 0. It is undefined behavior on x86 cpu.
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
/// `data` must not be 0. It is undefined behavior on x86 cpu.
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

	#[ktest]
	fn size_to_rank_basic() {
		assert_eq!(size_to_rank(0), 0);
		assert_eq!(size_to_rank(1), 0);
		assert_eq!(size_to_rank(PAGE_SIZE), 0);
		assert_eq!(size_to_rank(PAGE_SIZE + 1), 1);
		assert_eq!(size_to_rank(PAGE_SIZE * 2), 1);
		assert_eq!(size_to_rank(PAGE_SIZE * 2 + 1), 2);
		assert_eq!(size_to_rank(PAGE_SIZE * 3), 2);
		assert_eq!(size_to_rank(PAGE_SIZE * 4), 2);
		assert_eq!(size_to_rank(PAGE_SIZE * 4 + 1), 3);
		assert_eq!(size_to_rank(PAGE_SIZE * 5), 3);
	}
}

pub fn level_of(layout: Layout) -> usize {
	let size = layout.size();
	let align = layout.align();

	if size <= 1 && align == 1 {
		return LEVEL_MIN;
	}

	let rank = unsafe {
		match size > align {
			true => bit_scan_reverse(size - 1) + 1,
			false => bit_scan_reverse(align - 1) + 1,
		}
	};

	LEVEL_MIN + rank.checked_sub(LEVEL_MIN).unwrap_or_default()
}

pub const fn align_with_hw_cache(bytes: usize) -> usize {
	const CACHE_LINE_SIZE: usize = 64; // L1

	match bytes {
		0..=16 => 16,
		17..=32 => 32,
		_ => CACHE_LINE_SIZE * ((bytes - 1) / CACHE_LINE_SIZE + 1),
	}
}
