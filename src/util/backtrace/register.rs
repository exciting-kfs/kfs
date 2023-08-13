/// Get the register value of "name" in current context.

#[macro_export]
macro_rules! register {
    ($name:literal) => {
        unsafe {
            let mut ret: usize;
            core::arch::asm!(
                concat!("mov {0}, ", $name),
                out(reg) ret,
                options(nostack)
            );
            ret
        }
    }
}
