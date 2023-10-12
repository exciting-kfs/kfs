#![no_std]

#[no_link]
extern crate kernel;

#[no_mangle]
pub extern "C" fn init_module() {
	let mut arr: [u8; 1024] = [0; 1024];

	arr[0] = 4;
	kernel::do_something();
	// kernel::pr_warn!("ARR[0]: {}", arr[0]);
}
