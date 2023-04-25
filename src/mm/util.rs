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
pub const fn current_or_next_aligned(p: usize, align: usize) -> usize {
	(p + align - 1) & !(align - 1)
}

#[inline]
pub const fn next_aligned(p: usize, align: usize) -> usize {
	(p + align) & !(align - 1)
}

#[inline]
pub const fn is_aligned(addr: usize, align: usize) -> bool {
	addr % align == 0
}
