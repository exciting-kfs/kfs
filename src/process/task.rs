use core::ptr::NonNull;
use core::{alloc::AllocError, mem::size_of};

use alloc::{collections::LinkedList, sync::Arc};

use crate::mm::alloc::{page::alloc_pages, Zone};
use crate::mm::page::arch::{CURRENT_PD, PD};

use crate::mm::util::*;
use crate::sync::locked::Locked;
use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

use crate::config::KSTACK_RANK;

pub static CURRENT: CpuLocal<Arc<Locked<Task>>> = CpuLocal::uninit();
pub static TASK_QUEUE: Singleton<LinkedList<Arc<Locked<Task>>>> = Singleton::uninit();

pub enum State {
	Ready,
	Running,
	Sleeping,
	Exited,
}

pub struct Task {
	pub state: State,
	pub kstack: Stack,
	pub pid: usize,
	pub page_dir: PD,
}

impl Task {
	pub fn alloc_new() -> Result<Arc<Locked<Self>>, AllocError> {
		let pd = CURRENT_PD.lock().clone()?;
		let kstack = Stack::alloc()?;

		Ok(Arc::new(Locked::new(Task {
			state: State::Ready,
			kstack,
			pid: 0,
			page_dir: pd,
		})))
	}

	pub fn esp_mut(&mut self) -> *mut *mut usize {
		self.kstack.esp_mut()
	}
}

const KSTACK_SIZE: usize = rank_to_size(KSTACK_RANK);
type StackStorage = [u8; KSTACK_SIZE];

#[repr(C)]
pub struct Stack {
	storage: NonNull<StackStorage>,
	esp: *mut usize,
}

impl Stack {
	pub fn alloc() -> Result<Self, AllocError> {
		let storage: NonNull<StackStorage> = alloc_pages(KSTACK_RANK, Zone::Normal)?.cast();

		let esp = (storage.as_ptr() as usize + size_of::<StackStorage>()) as *mut usize;

		Ok(Self { storage, esp })
	}

	pub fn esp_mut(&mut self) -> *mut *mut usize {
		&mut self.esp
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
}
