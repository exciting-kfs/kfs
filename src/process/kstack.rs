use core::alloc::AllocError;
use core::mem::{self, size_of};
use core::ptr::{addr_of_mut, NonNull};

use crate::config::KSTACK_RANK;
use crate::interrupt::InterruptFrame;
use crate::mm::alloc::page::{alloc_pages, free_pages};
use crate::mm::alloc::Zone;
use crate::mm::util::*;
use crate::x86::{get_eflags, DPL_USER, GDT};

use super::kthread::kthread_entry;
use super::task::return_from_fork;

const KSTACK_SIZE: usize = rank_to_size(KSTACK_RANK);
type StackStorage = [u8; KSTACK_SIZE];

#[derive(Debug)]
pub struct StackOverFlow;

#[repr(C)]
pub struct Stack {
	esp: *mut usize,
	storage: NonNull<StackStorage>,
}

impl Stack {
	pub fn alloc() -> Result<Self, AllocError> {
		let storage: NonNull<StackStorage> = alloc_pages(KSTACK_RANK, Zone::Normal)?.cast();

		let esp = (storage.as_ptr() as usize + size_of::<StackStorage>()) as *mut usize;

		Ok(Self { storage, esp })
	}

	pub fn new_kernel(routine: usize, arg: usize) -> Result<Self, AllocError> {
		let mut stack = Self::alloc()?;

		stack.push(arg).unwrap();
		stack.push(routine).unwrap();
		stack.push(0).unwrap();
		stack.push(kthread_entry as usize).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();

		Ok(stack)
	}

	pub fn new_user(user_return_addr: usize, user_stack: usize) -> Result<Self, AllocError> {
		let mut stack = Self::alloc()?;

		let eflags = get_eflags() | (1 << 9); // enable interrupt

		// interrupt frame
		stack.push(GDT::USER_DATA | DPL_USER).unwrap();
		stack.push(user_stack).unwrap();
		stack.push(eflags).unwrap();
		stack.push(GDT::USER_CODE | DPL_USER).unwrap();
		stack.push(user_return_addr).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(GDT::USER_DATA | DPL_USER).unwrap();
		stack.push(GDT::USER_DATA | DPL_USER).unwrap();
		stack.push(GDT::USER_DATA | DPL_USER).unwrap();
		stack.push(GDT::USER_DATA | DPL_USER).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();

		// kernel context frame
		stack.push(return_from_fork as usize).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();

		Ok(stack)
	}

	pub fn clone_for_fork(&self, frame: *mut InterruptFrame) -> Result<Self, AllocError> {
		let mut new = Self::alloc()?;

		new.esp = new.as_interrupt_frame().cast();
		unsafe { new.as_interrupt_frame().copy_from_nonoverlapping(frame, 1) };
		unsafe { new.set_user_return_value(0) };

		new.push(return_from_fork as usize).unwrap();
		new.push(0).unwrap();
		new.push(0).unwrap();
		new.push(0).unwrap();
		new.push(0).unwrap();

		Ok(new)
	}

	pub unsafe fn from_raw(top: *mut StackStorage) -> Self {
		Self {
			esp: 0 as *mut usize,
			storage: NonNull::new_unchecked(top),
		}
	}

	fn esp_offset(&self) -> usize {
		self.esp as usize - self.storage.as_ptr() as usize
	}

	fn is_esp_in_bound(&self, esp: *const usize) -> bool {
		let distance = (esp as usize).checked_sub(self.storage.as_ptr() as usize);

		match distance {
			Some(x) => x < KSTACK_SIZE,
			None => false,
		}
	}

	pub fn push(&mut self, value: usize) -> Result<(), StackOverFlow> {
		let new_esp = unsafe { self.esp.sub(1) };

		if !self.is_esp_in_bound(new_esp) {
			return Err(StackOverFlow);
		}

		unsafe { (new_esp).write(value) };
		self.esp = new_esp;

		Ok(())
	}

	pub fn as_interrupt_frame(&self) -> *mut InterruptFrame {
		(self.base() - size_of::<InterruptFrame>()) as *mut InterruptFrame
	}

	pub unsafe fn set_user_return_value(&self, value: i32) {
		addr_of_mut!((*self.as_interrupt_frame()).eax).write(mem::transmute(value));
	}

	pub fn base(&self) -> usize {
		self.storage.as_ptr() as usize + KSTACK_SIZE
	}
}

impl Drop for Stack {
	fn drop(&mut self) {
		free_pages(self.storage.cast());
	}
}
