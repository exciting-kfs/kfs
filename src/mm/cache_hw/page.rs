
#[macro_export]
macro_rules! flush_page_cache {
	($directory:ident) => {
		asm!("mov cr3, eax", in("eax") &$directory);
	};
	() => {
		asm!("mov cr3, eax", in("eax") &GLOBAL_PD);
	}
}

#[macro_export]
macro_rules! flush_global_page_cache {
    () => {
	asm!(
		"mov eax, cr4",
		"or eax, 0x80", // PGE
		"mov cr4, eax"
	)
    };
}