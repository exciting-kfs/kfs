use alloc::boxed::Box;
use alloc::collections::LinkedList;
use core::arch::asm;
use core::mem::size_of;

use crate::mm::page::arch::{CURRENT_PD, PD};
use crate::sync::singleton::Singleton;
use crate::{interrupt::apic::end_of_interrupt, mm::constant::MB, pr_info, sync::locked::Locked};

pub enum State {
	Ready,
	Running,
	Sleeping,
	Exited,
}

#[repr(C)]
pub struct Context {
	pub eip: u32,
	pub esp: u32,
}

pub struct Task<'a> {
	pub context: Context,
	pub state: State,
	pub kstack: Box<Stack>,
	pub pid: usize,
	pub page_dir: PD<'a>,
}

#[repr(C)]
pub struct Stack {
	_padd: [u8; 8192 - size_of::<InterruptFrame>()],
	pub frame: InterruptFrame,
}

impl Stack {
	pub fn new() -> Self {
		Self {
			_padd: [0; 8192 - size_of::<InterruptFrame>()],
			frame: InterruptFrame::default(),
		}
	}
}

impl<'a> Task<'a> {
	#[inline(never)]
	pub fn new() -> Task<'a> {
		let kstack = Box::<Stack>::new(Stack::new());

		let context = Context {
			eip: return_from_interrupt as usize as u32,
			esp: &kstack.frame as *const _ as usize as u32,
		};

		let pd = CURRENT_PD.lock().clone().unwrap();

		let mut task = Task {
			state: State::Ready,
			context,
			kstack,
			pid: 0,
			page_dir: pd,
		};

		task
	}
}

static CURRENT: Singleton<Box<Task>> = Singleton::uninit();
static TASK_QUEUE: Singleton<LinkedList<Box<Task>>> = Singleton::uninit();

#[derive(Debug, Default)]
#[repr(C)]
pub struct InterruptFrame {
	pub ebp: u32,
	pub edi: u32,
	pub esi: u32,
	pub edx: u32,
	pub ecx: u32,
	pub ebx: u32,
	pub eax: u32,
	pub ds: u32,
	pub es: u32,
	pub fs: u32,
	pub gs: u32,

	// additional informations
	pub handler: u32,
	pub error_code: u32,

	// automatically set by cpu
	pub eip: u32,
	pub cs: u32,
	pub eflags: u32,

	// may not exist
	pub esp: u32,
	pub ss: u32,
}

pub extern "C" fn repeat_x(x: usize) -> ! {
	loop {
		pr_info!("Message from {}", x);
		unsafe { asm!("hlt") }
	}
}

#[no_mangle]
pub unsafe extern "C" fn do_handle_timer(frame: &InterruptFrame) {
	end_of_interrupt();

	let mut task_q = TASK_QUEUE.lock();
	let mut current = CURRENT.lock();

	let prev = &mut *current;
	let next = task_q.pop_front().unwrap();

	task_q.push_back(next);

	let next = task_q.back_mut().unwrap();

	core::mem::swap(prev, next);

	switch_process(&mut next.context, &mut prev.context);
}

pub extern "C" fn kthread_create<'a>(main: usize, arg: usize) -> Box<Task<'a>> {
	let mut task = Box::new(Task::new());

	task.context.eip = main as usize as u32;

	let mut bottom = (&task.kstack._padd[0] as *const _ as usize as u32) + 8192;

	unsafe {
		bottom -= 4;
		*(bottom as *mut usize) = arg;
		bottom -= 4;
		*(bottom as *mut usize) = 0;
	}

	task.context.esp = bottom;

	task
}

extern "C" {
	pub fn handle_timer();
	pub fn switch_process(prev: &mut Context, next: &mut Context);
	pub fn return_from_interrupt();
}

pub fn user_main(argc: i32, argv: &[&[u8]]) -> i32 {
	return 1;
}

pub unsafe extern "C" fn start() -> ! {
	let a = kthread_create(repeat_x as usize, 0);
	let b = kthread_create(repeat_x as usize, 1);

	TASK_QUEUE.write(LinkedList::new());
	TASK_QUEUE.lock().push_back(a);

	CURRENT.write(b);

	let mut dummy = Context { eip: 0, esp: 0 };

	switch_process(&mut dummy, &mut CURRENT.lock().context);

	loop {
		asm!("hlt")
	}
}
