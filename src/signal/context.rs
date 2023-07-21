use core::ptr::copy_nonoverlapping;

use crate::interrupt::InterruptFrame;

use super::sig_flag::SigFlag;

#[derive(Debug)]
#[repr(C)]
pub struct SigContext {
	pub intr: InterruptFrame,
	pub mask: SigFlag,
}

// struct sigcontext {
//   ...
//   struct _fpstate * fpstate;
//   unsigned long cr2;
// };

impl SigContext {
	pub fn new(intr_frame: *const InterruptFrame, mask: SigFlag) -> Self {
		let mut intr = InterruptFrame::empty();
		unsafe { copy_nonoverlapping(intr_frame, &mut intr as *mut InterruptFrame, 1) }

		Self { intr, mask }
	}
}
