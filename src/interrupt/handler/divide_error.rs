use crate::{pr_info, pr_warn};

use super::interrupt_info::InterruptInfo;

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
pub extern "x86-interrupt" fn divide_error_handler(info: InterruptInfo) {
	pr_warn!("fault: divide error");

	pr_info!("{:x?}", info);

	loop {}
}
