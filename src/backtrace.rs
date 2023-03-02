mod frame_pointer;

use core::arch::asm;
use frame_pointer::FramePointer;

pub struct Backtrace {
    fp: FramePointer
}

impl Backtrace {
    pub fn new() -> Self {
        Backtrace {
            fp: FramePointer::new()
        }
    }
}