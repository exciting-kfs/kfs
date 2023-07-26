use crate::interrupt::InterruptFrame;

use super::sig_mask::SigMask;

#[derive(Debug)]
#[repr(C)]
pub struct SigCtx {
	pub intr_frame: InterruptFrame,
	pub mask: SigMask,
	pub syscall_ret: isize,
}

// struct sigcontext {
//   ...
//   struct _fpstate * fpstate;
//   unsigned long cr2;
// };

impl SigCtx {
	pub fn new(intr_frame: &InterruptFrame, mask: SigMask, syscall_ret: isize) -> Self {
		Self {
			intr_frame: intr_frame.clone(),
			mask,
			syscall_ret,
		}
	}
}
