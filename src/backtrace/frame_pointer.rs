use core::arch::asm;

pub struct FramePointer {
    ptr: *const usize,
    func_addr: *const usize
}

impl FramePointer {

    pub fn new() -> Self {
        let ptr = frame_pointer().next().next();
        let func_addr = function_address(ptr);

        FramePointer {
            ptr: ptr.next(),
            func_addr,
        }
    }

    pub fn next(&self) -> Option<Self> {
        let func_addr = function_address(self.ptr);
        let ptr = self.ptr.next();

        Some(FramePointer { ptr, func_addr })
    }
}



fn function_address(prev_fp: *const usize) -> *const usize {
    let return_addr: isize;
    let offset: isize;

    unsafe {
        asm!(
            "mov {1}, [{0}+4]",
            "mov {2}, [{1}-4]",
            in(reg) prev_fp as usize,
            out(reg) return_addr,
            out(reg) offset,
            options(nostack)
        );
    }
    (return_addr + offset) as *const usize
}


fn frame_pointer() -> *const usize {
    let frame_pointer: usize;

    unsafe {
        asm!(
            "mov {0}, [ebp]",
            out(reg) frame_pointer,
            options(nostack)
        )
    }
    frame_pointer as *const usize
}


trait FindnextFrame {
    fn next(self) -> *const usize;
}

impl FindnextFrame for *const usize {

    fn next(self) -> *const usize {
        let frame_pointer: usize;
    
        unsafe {
            asm!(
                "mov {1}, [{0}]",
                in(reg) self as usize,
                out(reg) frame_pointer,
                options(nostack)
            )
        }
        frame_pointer as *const usize
    }    
}