use core::ptr::NonNull;
use core::{alloc::AllocError, mem::size_of};

use alloc::{collections::LinkedList, sync::Arc};
use kfs_macro::context;

use crate::mm::alloc::{page::alloc_pages, Zone};
use crate::mm::page::{KERNEL_PD, PD};

use crate::mm::util::*;
use crate::sync::locked::Locked;
use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

use crate::config::KSTACK_RANK;

use super::context::switch_stack;

pub static CURRENT: CpuLocal<Arc<Task>> = CpuLocal::uninit();
pub static TASK_QUEUE: Singleton<LinkedList<Arc<Task>>> = Singleton::new(LinkedList::new());

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
	Ready,
	Running,
	Sleeping,
	Exited,
}

#[repr(C)]
pub struct Task {
	pub kstack: Stack,
	pub state: Locked<State>,
	pub page_dir: PD,
}

impl Task {
	pub fn alloc_new(kstack: Stack) -> Result<Arc<Self>, AllocError> {
		let pd = KERNEL_PD.lock().clone()?;

		Ok(Arc::new(Task {
			state: Locked::new(State::Ready),
			kstack,
			page_dir: pd,
		}))
	}
}
const KSTACK_SIZE: usize = rank_to_size(KSTACK_RANK);
type StackStorage = [u8; KSTACK_SIZE];

#[repr(C)]
pub struct Stack {
	pub esp: *mut usize,
	storage: NonNull<StackStorage>,
	is_storage_external: bool,
}

impl Stack {
	pub fn alloc() -> Result<Self, AllocError> {
		let storage: NonNull<StackStorage> = alloc_pages(KSTACK_RANK, Zone::Normal)?.cast();

		let esp = (storage.as_ptr() as usize + size_of::<StackStorage>()) as *mut usize;

		Ok(Self {
			storage,
			esp,
			is_storage_external: false,
		})
	}

	pub unsafe fn from_raw(top: *mut StackStorage) -> Self {
		Self {
			esp: 0 as *mut usize,
			storage: NonNull::new_unchecked(top),
			is_storage_external: true,
		}
	}

	fn is_esp_in_bound(&self, esp: *const usize) -> bool {
		let distance = (esp as usize).checked_sub(self.storage.as_ptr() as usize);

		match distance {
			Some(x) => x < KSTACK_SIZE,
			None => false,
		}
	}

	pub fn push(&mut self, value: usize) -> Result<(), ()> {
		let new_esp = unsafe { self.esp.sub(1) };

		if self.is_esp_in_bound(new_esp) {
			unsafe { (new_esp).write(value) };
			self.esp = new_esp;

			Ok(())
		} else {
			Err(())
		}
	}

	pub fn base(&self) -> usize {
		self.storage.as_ptr() as usize + KSTACK_SIZE
	}
}

#[context(irq_disabled)]
pub fn yield_now() {
	let next = {
		let mut task_q = TASK_QUEUE.lock();

		match task_q.pop_front() {
			Some(x) => x,
			None => return,
		}
	};

	// safety: this function always called through interrupt gate.
	// so IRQ is disabled.
	let curr = unsafe { CURRENT.get_mut() }.clone();

	let curr_task = Arc::into_raw(curr);
	let next_task = Arc::into_raw(next);

	// TODO: check this
	unsafe { switch_stack(curr_task, next_task) };
}
