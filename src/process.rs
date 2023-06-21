use alloc::boxed::Box;
use alloc::collections::LinkedList;
use core::alloc::AllocError;
use core::arch::asm;
use core::mem::size_of;

use crate::interrupt::{irq_enable, irq_stack_restore, InterruptFrame};
use crate::mm::alloc::page::alloc_pages;
use crate::mm::alloc::Zone;
use crate::mm::constant::PAGE_SIZE;
use crate::mm::page::arch::{CURRENT_PD, PD};
use crate::sync::cpu_local::CpuLocal;
use crate::sync::singleton::Singleton;
use crate::{interrupt::apic::end_of_interrupt, pr_info};

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

type StackStorage = [u8; 2 * PAGE_SIZE];

#[repr(C)]
pub struct Stack<'a> {
	storage: &'a mut StackStorage,
	esp: usize,
}

impl<'a> Stack<'a> {
	pub fn new() -> Result<Self, AllocError> {
		let storage: &'a mut StackStorage = unsafe {
			alloc_pages(1, Zone::Normal)?
				.cast::<StackStorage>()
				.as_ptr()
				.as_mut()
				.unwrap()
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

impl<'a> Task<'a> {
	pub fn new() -> Result<Self, AllocError> {
		let pd = CURRENT_PD.lock().clone()?;
		let kstack = Stack::new()?;

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

static CURRENT: CpuLocal<Box<Task>> = CpuLocal::uninit();
static TASK_QUEUE: Singleton<LinkedList<Box<Task>>> = Singleton::uninit();

pub extern "C" fn repeat_x(x: usize) -> ! {
	loop {
		pr_info!("FROM X={}", x);
		unsafe { asm!("hlt") }
	}
}

#[no_mangle]
pub unsafe extern "C" fn handle_timer_impl(_frame: &InterruptFrame) {
	end_of_interrupt();

	let mut task_q = TASK_QUEUE.lock();
	let mut current = CURRENT.get_mut();

	let prev = &mut *current;
	let next = task_q.pop_front().unwrap();

	task_q.push_back(next);

	let next = task_q.back_mut().unwrap();

	core::mem::swap(prev, next);

	switch_stack(next.esp_mut(), prev.esp_mut());
}

pub unsafe extern "C" fn kthread_exec_cleanup(callback: extern "C" fn(usize) -> !, arg: usize) {
	unsafe { TASK_QUEUE.manual_unlock() };
	irq_stack_restore();
	irq_enable();

	callback(arg);
}

pub fn kthread_create<'a>(main: usize, arg: usize) -> Result<Box<Task<'a>>, AllocError> {
	let mut task = Box::new(Task::new()?);

	task.kstack.push(arg);
	task.kstack.push(main);
	task.kstack.push(41);
	task.kstack.push(kthread_exec_cleanup as usize);
	task.kstack.push(42);
	task.kstack.push(43);
	task.kstack.push(44);
	task.kstack.push(45);

	Ok(task)
}

extern "C" {
	pub fn handle_timer();
	pub fn switch_stack(prev_stack: &mut usize, next_stack: &mut usize);
	pub fn kthread_exec(esp: usize) -> !;
}

pub unsafe extern "C" fn scheduler() -> ! {
	let a = kthread_create(repeat_x as usize, 1111).expect("OOM");
	let b = kthread_create(repeat_x as usize, 2222).expect("OOM");
	let c = kthread_create(repeat_x as usize, 3333).expect("OOM");

	TASK_QUEUE.write(LinkedList::new());
	TASK_QUEUE.lock().push_back(b);
	TASK_QUEUE.lock().push_back(c);

	CURRENT.init(a);

	kthread_exec(*CURRENT.get_mut().esp_mut());
}
