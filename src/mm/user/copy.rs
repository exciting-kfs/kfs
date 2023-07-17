use core::alloc::AllocError;

use crate::mm::{
	alloc::virt::{kmap, kunmap},
	constant::PAGE_SIZE,
};

pub unsafe fn copy_to_user_page(kernel_vaddr: usize, user_paddr: usize) -> Result<(), AllocError> {
	let user_ptr = kmap(user_paddr)?;

	user_ptr
		.as_ptr()
		.copy_from_nonoverlapping(kernel_vaddr as *const u8, PAGE_SIZE);

	kunmap(user_ptr.as_ptr() as usize);

	Ok(())
}

pub unsafe fn copy_user_to_user_page(
	user_src_paddr: usize,
	user_dst_paddr: usize,
) -> Result<(), AllocError> {
	let temp_src_ptr = kmap(user_src_paddr)?;
	let temp_dst_ptr = match kmap(user_dst_paddr) {
		Ok(x) => x,
		Err(e) => {
			kunmap(temp_src_ptr.as_ptr() as usize);
			return Err(e);
		}
	};

	temp_dst_ptr
		.as_ptr()
		.copy_from_nonoverlapping(temp_src_ptr.as_ptr(), PAGE_SIZE);

	kunmap(temp_dst_ptr.as_ptr() as usize);
	kunmap(temp_src_ptr.as_ptr() as usize);

	Ok(())
}

pub unsafe fn copy_from_user_page(
	user_paddr: usize,
	kernel_vaddr: usize,
) -> Result<(), AllocError> {
	let user_ptr = kmap(user_paddr)?;

	user_ptr
		.as_ptr()
		.copy_to_nonoverlapping(kernel_vaddr as *mut u8, PAGE_SIZE);

	kunmap(user_ptr.as_ptr() as usize);

	Ok(())
}

pub unsafe fn memset_to_user_page(user_paddr: usize, value: u8) -> Result<(), AllocError> {
	let user_ptr = kmap(user_paddr)?;

	user_ptr.as_ptr().write_bytes(value, PAGE_SIZE);

	kunmap(user_ptr.as_ptr() as usize);

	Ok(())
}
