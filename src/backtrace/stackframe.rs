use core::arch::asm;

/// The type that holds informations of the stack frame.
pub struct Stackframe {
    pub(super) fn_addr: *const usize
}

impl Stackframe {
    pub fn new(base_ptr: *const usize) -> Self {
        let fa = func_addr_near(base_ptr);

        Stackframe {
            fn_addr: fa,
        }
    }
}

/// Get a function address of a stack frame.
fn func_addr_near(base_ptr: *const usize) -> *const usize {
    let ret_addr = return_address(base_ptr);
    let offset = offset_near(ret_addr);
    unsafe { ret_addr.offset(offset) as *const usize}
}

/// Get a return address in a stack frame.
fn return_address(base_ptr: *const usize) -> *const u8 {
    let ret: *const u8;
    unsafe {
        asm!(
            "mov {1}, [{0}]",
            in(reg) base_ptr.offset(1),
            out(reg) ret,
            options(nostack)
        )
    }
    ret
}

/// Get a function address offset from a return address using assembly 'near call'.
fn offset_near(ret_addr: *const u8) -> isize {
    let offset: isize;
    unsafe {
        asm!(
            "mov {1}, [{0} - 4]",
            in(reg) ret_addr,
            out(reg) offset,
            options(nostack)
        )
    }
    offset
}

/// Get the base pointer of the next frame.
pub fn next(base_ptr: *const usize) -> *const usize {
    let mut ret: *const usize;
    unsafe {
        asm!(
            "mov {0}, [{1}]",
            out(reg) ret,
            in(reg) base_ptr,
            options(nostack)
        );
        ret
    }
}
