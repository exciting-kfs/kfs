use crate::{interrupt::interrupt_info::InterruptInfo, pr_info, pr_warn};

/// # Initial Stack Frame
/// ```
/// addr | stack  | variable address
/// --------------------------------
/// low  | error  | <- error_code
///      | eip    | <- info
///      | cs     |
///      | eflags | <- esp (kernel) // privilege not changed
///      | esp    |
/// high | ss     | <- esp (user)   // privilege changed
/// --------------------------------
/// ```
pub extern "x86-interrupt" fn page_fault_handler(info: InterruptInfo, error_code: usize) {
	pr_warn!("fault: page error");
	pr_info!("{:x?}", info);
	pr_info!("{:x?}", error_code);

	loop {}
}
