use crate::interrupt::InterruptFrame;

use super::sig_mask::SigMask;

#[derive(Debug)]
#[repr(C)]
pub struct SigContext {
	pub intr: InterruptFrame,
	pub mask: SigMask,
}

// struct sigcontext {
//   ...
//   struct _fpstate * fpstate;
//   unsigned long cr2;
// };

impl SigContext {
	pub fn new(intr_frame: *const InterruptFrame, mask: SigMask) -> Self {
		Self {
			intr: unsafe { *intr_frame },
			mask,
		}
	}
}
