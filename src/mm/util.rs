use super::constant::VM_OFFSET;

#[inline]
pub const fn phys_to_virt(addr: usize) -> usize {
	addr.wrapping_add(VM_OFFSET)
}

#[inline]
pub const fn virt_to_phys(addr: usize) -> usize {
	addr.wrapping_sub(VM_OFFSET)
}

#[inline]
pub const fn to_phys(addr: usize) -> usize {
	addr - match addr >= VM_OFFSET {
		true => VM_OFFSET,
		false => 0,
	}
}

#[inline]
pub const fn to_virt(addr: usize) -> usize {
	addr + match addr < VM_OFFSET {
		true => VM_OFFSET,
		false => 0,
	}
}

#[inline]
pub const fn to_phys_64(addr: u64) -> u64 {
	addr - match addr >= VM_OFFSET as u64 {
		true => VM_OFFSET as u64,
		false => 0,
	}
}

#[inline]
pub const fn to_virt_64(addr: u64) -> u64 {
	addr + match addr < VM_OFFSET as u64 {
		true => VM_OFFSET as u64,
		false => 0,
	}
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
