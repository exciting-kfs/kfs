use super::PD;
use core::arch::asm;

/// Reload cr3 register.
/// note that this also invalidates all tlb entries except marked as global.
pub unsafe fn reload_cr3(page_directory: &PD) {
	asm!("mov cr3, eax", in("eax") page_directory);
}

/// Invalidate all tlb (global page included).
pub fn invalidate_all_tlb() {
	unsafe {
		asm!(
			"mov eax, cr4",
			"xor eax, 0x80", // PGE
			"mov cr4, eax",
			"or eax, 0x80",
			"mov cr4, eax"
		)
	};
}
