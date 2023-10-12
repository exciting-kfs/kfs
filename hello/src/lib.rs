#![no_std]

#[no_link]
extern crate kernel;

pub fn ini_module() {
	kernel::do_something();
}
