use super::constant::*;

/// convert address to page frame number
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

pub const fn size_to_rank(mut size: usize) -> usize {
	// TODO: O(CONSTANT) implementation? or hardware acceleration?
	size >>= PAGE_SHIFT;

	let mut rank = 0;
	while size > 0 && rank <= MAX_RANK {
		rank += 1;
		size >>= 1;
	}

	rank
}
