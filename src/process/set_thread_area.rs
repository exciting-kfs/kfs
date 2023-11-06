use crate::{
	mm::user::verify::verify_ptr_mut,
	syscall::errno::Errno,
	x86::{UserDesc, CPU_GDT},
};

use super::task::CURRENT;

pub fn sys_set_thread_area(user_desc: usize) -> Result<usize, Errno> {
	let current = unsafe { CURRENT.get_ref() };

	let user_desc = verify_ptr_mut::<UserDesc>(user_desc, current)?;

	let system_desc = user_desc.parse_into_system_desc()?;

	let idx = match user_desc.entry_number {
		-1 => 0 as usize,
		x @ 6..=8 => x as usize - 6,
		_ => return Err(Errno::EINVAL),
	};

	let gdt = unsafe { CPU_GDT.get_mut() };

	{
		let mut tls = current.get_user_ext().unwrap().lock_tls();
		gdt.set_tls_by_idx(idx, system_desc)?;
		tls[idx] = system_desc;

		gdt.pick_up();
	}

	user_desc.entry_number = idx as i32 + 6;

	Ok(0)
}
