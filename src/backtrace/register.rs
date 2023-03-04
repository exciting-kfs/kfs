#[macro_export]
macro_rules! register {
    ($arg:literal) => {
        unsafe {
            let mut ret: usize;
            core::arch::asm!(
                concat!("mov {0}, ", $arg),
                out(reg) ret,
                options(nostack)
            );
            ret
        }
    }
}