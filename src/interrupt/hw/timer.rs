use crate::{interrupt::interrupt_info::InterruptInfo, pr_info, pr_warn};

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
pub extern "x86-interrupt" fn handler(info: InterruptInfo) {
	pr_warn!("timer");

	pr_info!("{:x?}", info);

	loop {}
}
