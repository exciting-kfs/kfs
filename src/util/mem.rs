pub fn copy(dst: *mut u8, src: *const u8, len: usize) {
	unsafe {
		if !(src > dst.offset(len as isize) || src.offset(len as isize) < dst) {
			panic!("overlap");
		}
	}

	for i in 0..len {
		unsafe { dst.add(i).write_volatile(src.add(i).read_volatile()) };
	}
}
