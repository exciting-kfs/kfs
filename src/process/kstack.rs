use core::alloc::AllocError;
use core::mem::{self, size_of};
use core::ptr::{addr_of_mut, NonNull};

use crate::config::KSTACK_RANK;
use crate::interrupt::{leave_interrupt_context, InterruptFrame};
use crate::mm::alloc::page::{alloc_pages, free_pages};
use crate::mm::alloc::Zone;
use crate::mm::util::*;

use super::kthread::kthread_entry;

const KSTACK_SIZE: usize = rank_to_size(KSTACK_RANK);
type StackStorage = [u8; KSTACK_SIZE];

extern "C" {
	pub fn return_from_interrupt();
}

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

	pub fn push_interrupt_frame(&mut self, frame: &InterruptFrame) -> Result<(), StackOverFlow> {
		let new_esp = unsafe {
			self.esp
				.sub(size_of::<InterruptFrame>() / size_of::<usize>())
		};

		if !self.is_esp_in_bound(new_esp) {
			return Err(StackOverFlow);
		}

		unsafe {
			new_esp
				.cast::<InterruptFrame>()
				.copy_from_nonoverlapping(frame, 1)
		};

		self.esp = new_esp;

		Ok(())
	}

	pub fn new_user(user_return_addr: usize, user_stack: usize) -> Result<Self, AllocError> {
		let mut stack = Self::alloc()?;

		stack
			.push_interrupt_frame(&InterruptFrame::new_user(user_return_addr, user_stack))
			.unwrap();

		// kernel context frame
		stack.push(return_from_interrupt as usize).unwrap();
		stack.push(leave_interrupt_context as usize).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();
		stack.push(0).unwrap();

		Ok(stack)
	}

	pub fn clone_for_fork(&self, frame: *const InterruptFrame) -> Result<Self, AllocError> {
		let mut new = Self::alloc()?;

		new.esp = new.as_interrupt_frame().cast();
		unsafe { new.as_interrupt_frame().copy_from_nonoverlapping(frame, 1) };
		unsafe { new.set_user_return_value(0) };

		new.push(return_from_interrupt as usize).unwrap();
		new.push(leave_interrupt_context as usize).unwrap();
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
