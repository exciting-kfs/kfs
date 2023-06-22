use core::{alloc::AllocError, mem::size_of};

use alloc::{boxed::Box, collections::LinkedList};

use crate::mm::alloc::{page::alloc_pages, Zone};
use crate::mm::constant::PAGE_SIZE;
use crate::mm::page::arch::{CURRENT_PD, PD};

use crate::sync::{cpu_local::CpuLocal, singleton::Singleton};

pub static CURRENT: CpuLocal<Box<Task>> = CpuLocal::uninit();
pub static TASK_QUEUE: Singleton<LinkedList<Box<Task>>> = Singleton::uninit();

pub enum State {
	Ready,
	Running,
	Sleeping,
	Exited,
}

pub struct Task<'a> {
	pub state: State,
	pub kstack: Stack<'a>,
	pub pid: usize,
	pub page_dir: PD<'a>,
}

impl<'a> Task<'a> {
	pub fn alloc() -> Result<Self, AllocError> {
		let pd = CURRENT_PD.lock().clone()?;
		let kstack = Stack::alloc()?;

		Ok(Task {
			state: State::Ready,
			kstack,
			pid: 0,
			page_dir: pd,
		})
	}

	pub fn esp_mut(&mut self) -> &mut usize {
		self.kstack.esp_mut()
	}
}

type StackStorage = [u8; 2 * PAGE_SIZE];

#[repr(C)]
pub struct Stack<'a> {
	storage: &'a mut StackStorage,
	esp: usize,
}

impl<'a> Stack<'a> {
	pub fn alloc() -> Result<Self, AllocError> {
		let storage: &'a mut StackStorage = unsafe {
			alloc_pages(1, Zone::Normal)?
				.cast::<StackStorage>()
				.as_mut()
		};

		let esp = storage as *const _ as usize + size_of::<StackStorage>();

		Ok(Self { storage, esp })
	}

	pub fn esp_mut(&mut self) -> &mut usize {
		&mut self.esp
	}

	pub fn push(&mut self, value: usize) {
		self.esp -= 4;
		unsafe { (self.esp as *mut usize).write(value) };
	}
}
