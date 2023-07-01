//! early stage user space

use core::{arch::asm, ptr::NonNull};

use crate::{
	interrupt::InterruptFrame,
	mm::{
		alloc::{page, Zone},
		constant::PAGE_SIZE,
		page::{map_mmio, PageFlag},
		util::{size_to_rank, virt_to_phys},
	},
	process::context::{context_switch, InContext},
	x86::{CPU_STACK, DPL_USER, GDT},
};

extern "C" {
	fn user_start();
	fn user_end();
}

const USER_SPACE_BEGIN: usize = 0xb000_0000;
const TEMP_SPACE_BEGIN: usize = 0xa000_0000;

pub unsafe fn copy_to_user(from: *const u8, mut ptr: NonNull<[u8]>, size: usize) {
	let slice = ptr.as_mut();
	let raw_ptr = slice.as_ptr();
	let ptr_len = slice.len();

	let mut remain = size;
	for x in 0..(ptr_len / PAGE_SIZE) {
		let dst = raw_ptr.add(x * PAGE_SIZE);
		let src = from.add(x * PAGE_SIZE);

		map_mmio(
			TEMP_SPACE_BEGIN,
			virt_to_phys(dst as usize),
			PageFlag::Present | PageFlag::Write,
		)
		.unwrap();

		map_mmio(
			USER_SPACE_BEGIN + x * PAGE_SIZE,
			virt_to_phys(dst as usize),
			PageFlag::Present | PageFlag::Write | PageFlag::User,
		)
		.unwrap();

		(TEMP_SPACE_BEGIN as *mut u8).write_bytes(0, PAGE_SIZE);
		(TEMP_SPACE_BEGIN as *mut u8).copy_from_nonoverlapping(src, PAGE_SIZE.min(remain));
		remain -= PAGE_SIZE.min(remain);

		// TODO: TEMP_SPACE TLB flushing
	}
}

extern "C" {
	fn user_process_exec(esp: *mut usize) -> !;
}

pub unsafe fn exec_user_space() -> ! {
	let total_size = user_end as usize - user_start as usize;

	let storage = page::alloc_pages(size_to_rank(total_size), Zone::High).unwrap();

	copy_to_user(user_start as usize as *mut u8, storage, total_size);

	let mut new_eflags: usize;
	asm!("pushfd", "pop {eflags}", eflags = out(reg) new_eflags);

	// set IF (enable interrupt)
	new_eflags |= 1 << 9;

	context_switch(InContext::IrqDisabled);
	let stack = CPU_STACK.get_mut();

	stack
		.as_mut_ptr()
		.cast::<InterruptFrame>()
		.write(InterruptFrame {
			ebp: 0,
			edi: 0,
			esi: 0,
			edx: 0,
			ecx: 0,
			ebx: 42,
			eax: 0,
			ds: GDT::USER_DATA | DPL_USER,
			es: GDT::USER_DATA | DPL_USER,
			fs: GDT::USER_DATA | DPL_USER,
			gs: GDT::USER_DATA | DPL_USER,
			handler: 0,
			error_code: 0,
			eip: USER_SPACE_BEGIN,
			cs: GDT::USER_CODE | DPL_USER,
			eflags: new_eflags,
			esp: 0,
			ss: GDT::USER_DATA | DPL_USER,
		});

	unsafe { user_process_exec((&*stack) as *const _ as usize as *mut usize) }
}
