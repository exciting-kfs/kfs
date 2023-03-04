use core::arch::asm;

pub struct Stackframe {
    pub(super) base_ptr: *const usize, // ?
    pub(super) fn_addr: *const usize
}

impl Stackframe {
    pub fn new(base_ptr: *const usize) -> Self {
        let fa = func_addr_near(base_ptr);

        Stackframe {
            base_ptr,
            fn_addr: fa,
        }
    }
}

fn func_addr_near(base_ptr: *const usize) -> *const usize {
    let ret_addr = return_address(base_ptr);
    let offset = offset_near(ret_addr);
    unsafe { ret_addr.offset(offset) as *const usize}
}

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
