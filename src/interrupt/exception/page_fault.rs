use core::fmt::{self, Display};

use bitflags::bitflags;
use kfs_macro::interrupt_handler;

use crate::driver::terminal::sys_attach_tty;
use crate::interrupt::InterruptFrame;
use crate::mm::alloc::virt::{kmap, kunmap};
use crate::mm::alloc::Zone;
use crate::mm::constant::PAGE_MASK;
use crate::mm::page::PageFlag;
use crate::mm::user::vma::{AreaFlag, UserAddressSpace};
use crate::process::exit::exit_with_signal;
use crate::process::signal::sig_num::SigNum;
use crate::process::task::CURRENT;
use crate::ptr::PageBox;
use crate::PAGE_SIZE;
use crate::{pr_err, pr_info, register};

bitflags! {
	#[derive(Clone, Copy)]
	pub struct ErrorCode: u32 {
		const Present = 1;
		const Write = 2;
		const User = 4;
		const Reserved = 8;
		const InstructionFetch = 16;
		const ProtectionKey = 32;
		const ShadowStack = 64;
		const HLAT = 128;
		const SGX = 256;
	}
}

impl Display for ErrorCode {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(
			f,
			concat!(
				"PRESENT: {} | ACCESS: {} | PRIV: {}\n",
				"RESV:    {} | FETCH:  {} | PK:   {}\n",
				"SS:      {} | HLAT:   {} | SGX:  {}"
			),
			self.contains(ErrorCode::Present) as u8,
			if self.contains(ErrorCode::Write) {
				'W'
			} else {
				'R'
			},
			if self.contains(ErrorCode::User) {
				'U'
			} else {
				'S'
			},
			self.contains(ErrorCode::Reserved) as u8,
			self.contains(ErrorCode::InstructionFetch) as u8,
			self.contains(ErrorCode::ProtectionKey) as u8,
			self.contains(ErrorCode::ShadowStack) as u8,
			self.contains(ErrorCode::HLAT) as u8,
			self.contains(ErrorCode::SGX) as u8,
		)
	}
}

fn lookup_page_info(
	vma: &UserAddressSpace,
	vaddr: usize,
	flags: AreaFlag,
) -> Result<(usize, PageFlag), ()> {
	let area = vma.find_area(vaddr).ok_or(())?;

	if !area.flags.contains(flags) {
		return Err(());
	}

	let base = vaddr & PAGE_MASK;
	let extra_flag = if area.flags.contains(AreaFlag::Writable) {
		PageFlag::Write
	} else {
		PageFlag::empty()
	};

	Ok((base, PageFlag::Present | PageFlag::User | extra_flag))
}

fn handle_user_page_fault(vaddr: usize, error_code: ErrorCode) -> Result<(), ()> {
	let current = unsafe { CURRENT.get_mut() };

	let flags = if error_code.contains(ErrorCode::Write) {
		AreaFlag::Writable
	} else {
		AreaFlag::Readable
	};

	let mut memory = current
		.get_user_ext()
		.expect("must be user task")
		.lock_memory();

	let page = PageBox::new(Zone::High).map_err(|_| ())?;

	let temp = kmap(page.as_phys_addr()).map_err(|_| ())?;
	unsafe { temp.as_ptr().write_bytes(0, PAGE_SIZE) };
	kunmap(temp.as_ptr() as usize);

	let (base, page_flags) = lookup_page_info(memory.get_vma(), vaddr, flags)?;

	memory
		.get_pd()
		.map_user(base, page.as_phys_addr(), page_flags)
		.map_err(|_| ())?;

	page.forget();

	Ok(())
}

#[interrupt_handler]
pub extern "C" fn handle_page_fault_impl(frame: InterruptFrame) {
	let addr = register!("cr2");
	let error_code = ErrorCode::from_bits_truncate(frame.error_code as u32);

	if let Ok(_) = handle_user_page_fault(addr, error_code) {
		return;
	}

	pr_err!("Exception(fault): PAGE FAULT");
	pr_info!("{}", frame);
	pr_info!("note: while accessing {:#0x}", addr);
	pr_info!("[DETAILED ERROR CODE]\n{}", error_code);

	if frame.is_user() {
		_ = sys_attach_tty();
		exit_with_signal(SigNum::SEGV);
	}

	// BUG
	loop {}
}
