use super::x86_page::PD;
use core::arch::asm;

/// Reload cr3 register.
/// note that this also invalidates all tlb entries except marked as global.
pub unsafe fn reload_cr3(page_directory: &PD) {
	asm!("mov cr3, eax", in("eax") page_directory);
}
