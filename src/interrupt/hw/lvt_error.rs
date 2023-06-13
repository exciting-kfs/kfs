use crate::{interrupt::interrupt_info::InterruptInfo, pr_info, pr_warn};

pub extern "x86-interrupt" fn handler(info: InterruptInfo) {
	pr_warn!("LVT Error");
	pr_info!("{:?}", info);
}
