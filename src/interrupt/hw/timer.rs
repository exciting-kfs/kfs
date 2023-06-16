use crate::interrupt::{
	apic::{self},
	interrupt_info::InterruptInfo,
};

static mut JIFFIES: usize = 0;

/// # Initial Stack Frame
/// ```
/// addr | stack  | variable address
/// --------------------------------
/// low  | eip    | <- info
///      | cs     |
///      | eflags | <- esp (kernel) // privilege not changed
///      | esp    |
/// high | ss     | <- esp (user)   // privilege changed
/// --------------------------------
/// ```
pub extern "x86-interrupt" fn handler(_: InterruptInfo) {
	unsafe { JIFFIES += 1 };
	// pr_info!("timer");
	apic::local_eoi();
}

pub fn get_jiffies() -> usize {
	unsafe { JIFFIES }
}
