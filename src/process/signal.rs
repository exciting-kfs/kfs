pub mod sig_code;
pub mod sig_ctx;
pub mod sig_flag;
pub mod sig_handler;
pub mod sig_info;
pub mod sig_mask;
pub mod sig_num;

use core::{
	array,
	mem::{self, size_of, variant_count},
	ptr::copy_nonoverlapping,
};

use alloc::{
	collections::LinkedList,
	sync::{Arc, Weak},
};

use crate::{
	config::TRAMPOLINE_BASE,
	interrupt::InterruptFrame,
	pr_debug,
	process::{
		exit::exit_with_signal,
		relation::session::Session,
		signal::{sig_flag::SigFlag, sig_handler::SigHandler, sig_num::SigNum},
		task::{Task, CURRENT},
	},
	scheduler::sleep::{sleep_and_yield, wake_up, Sleep},
	sync::Locked,
	syscall::{errno::Errno, signal::is_syscall_restart},
};

use self::{
	sig_code::SigCode, sig_ctx::SigCtx, sig_handler::SigAction, sig_info::SigInfo,
	sig_mask::SigMask,
};

extern "C" {
	fn signal_trampoline();
	fn __trampoline_start();
	fn go_to_signal_handler(intr_frame: *const InterruptFrame, new_esp: usize, handler: usize)
		-> !;
}

pub fn trampoline_address(addr: usize) -> usize {
	TRAMPOLINE_BASE + (addr - __trampoline_start as usize)
}

pub struct Restart;
pub struct Signal {
	queue: Locked<LinkedList<SigInfo>>,
	mask: Locked<SigMask>,
	pub table: Locked<[SigHandler; variant_count::<SigNum>()]>,
}

impl Signal {
	pub fn new() -> Self {
		Self {
			mask: Locked::new(SigMask::empty()),
			queue: Locked::new(LinkedList::new()),
			table: Locked::new(array::from_fn(|i| {
				SigHandler::default(SigNum::from_usize(i + 1).unwrap())
			})),
		}
	}

	pub fn clone_for_fork(&self) -> Self {
		Self {
			queue: Locked::new(LinkedList::new()),
			mask: self.mask.clone(),
			table: self.table.clone(),
		}
	}

	pub fn overwrite_mask(&self, mask: SigMask) {
		*self.mask.lock() = mask;
	}

	pub fn recv_signal(&self, info: SigInfo) {
		let mask = self.mask.lock();
		if mask.contains(info.num.into()) {
			return;
		}

		let table = self.table.lock();
		if let SigHandler::Ignore = table[info.num.index()] {
			return;
		}

		let mut queue = self.queue.lock();

		use SigNum::*;
		match info.num {
			STOP | TSTP | TTIN | TTOU => {
				let _ = queue.extract_if(|info| info.num == CONT);
				queue.push_front(info);
			}
			CONT => {
				let _ = queue.extract_if(|info| info.num.is_stop());
				queue.push_front(info);
			}
			KILL => queue.push_front(info),
			_ => queue.push_back(info),
		}
	}

	pub fn do_signal(&self, frame: &InterruptFrame, syscall_ret: isize) -> Option<Restart> {
		let info = self.get_signal_event()?;
		let handler = self.get_handler(&info.num);
		match &handler {
			SigHandler::Some(act) => unsafe {
				let o_mask = self.replace_mask(act, &info);
				let sig_ctx = SigCtx::new(frame, o_mask, syscall_ret);
				self.do_action(act, &info, &sig_ctx)
			},
			x => self.do_signal_default(x, info.num),
		};

		is_syscall_restart(syscall_ret, handler.get_flag()).then_some(Restart)
	}

	pub fn do_signal_repeat(&self, frame: &InterruptFrame) -> Option<()> {
		let info = self.get_signal_event()?;
		let handler = self.get_handler(&info.num);
		match &handler {
			SigHandler::Some(act) => unsafe {
				self.replace_mask(act, &info);
				self.do_action_repeat(act, &info, frame);
			},
			x => self.do_signal_default(x, info.num),
		}
	}

	fn do_signal_default(&self, handler: &SigHandler, num: SigNum) -> Option<()> {
		use SigHandler::*;
		match handler {
			Terminate | Core => exit_with_signal(num),
			Ignore | Continue => Option::Some(()),
			Stop => {
				sleep_and_yield(Sleep::Deep);
				Option::Some(())
			}
			_ => unreachable!(),
		}
	}

	/// # Safety
	///
	/// Must clean up lock and global variable before this function call.
	unsafe fn do_action(&self, act: &SigAction, info: &SigInfo, ctx: &SigCtx) -> ! {
		let mut esp = ctx.intr_frame.esp;
		push_to_user_stack(
			&mut esp,
			ctx as *const SigCtx as *const u8,
			size_of::<SigCtx>(),
		);
		push_to_user_stack(
			&mut esp,
			info as *const SigInfo as *const u8,
			size_of::<SigInfo>(),
		);

		let mut func_frame = [0; 4];
		make_function_frame(&mut func_frame, esp, info.num);
		push_to_user_stack(
			&mut esp,
			func_frame.as_ptr().cast(),
			func_frame.len() * size_of::<usize>(),
		);
		// pr_debug!("sig_action: go_to_signal_handler: esp {:x}", esp);
		go_to_signal_handler(&ctx.intr_frame as *const InterruptFrame, esp, act.handler());
	}

	/// # Safety
	///
	/// Must clean up lock and global variable before this function call.
	unsafe fn do_action_repeat(
		&self,
		act: &SigAction,
		info: &SigInfo,
		frame: &InterruptFrame,
	) -> ! {
		let info_ptr = frame.ecx as *mut SigInfo;
		copy_nonoverlapping(info as *const SigInfo, info_ptr, 1);

		let mut esp = frame.esp;
		let trampoline = trampoline_address(signal_trampoline as usize);
		push_to_user_stack(
			&mut esp,
			&trampoline as *const usize as *const u8,
			size_of::<usize>(),
		);
		// pr_debug!("sig_action_repeat: go_to_signal_handler: esp {:x}", esp);
		go_to_signal_handler(frame as *const InterruptFrame, esp, act.handler());
	}

	fn replace_mask(&self, act: &SigAction, info: &SigInfo) -> SigMask {
		let mut lock = self.mask.lock();
		let o_mask = *lock;
		let n_mask = if act.flag().contains(SigFlag::NoDefer) {
			o_mask | act.mask() - info.num.into()
		} else {
			o_mask | act.mask() | info.num.into()
		};

		mem::replace(&mut *lock, n_mask)
	}

	fn get_signal_event(&self) -> Option<SigInfo> {
		let mask = self.mask.lock();
		let mut queue = self.queue.lock();

		let mut ret = None;
		let mut not = LinkedList::new();
		while let Some(info) = queue.pop_front() {
			if mask.contains(info.num.into()) {
				not.push_back(info)
			} else {
				ret = Some(info);
				break;
			}
		}
		queue.append(&mut not);

		ret
	}

	pub fn get_handler(&self, num: &SigNum) -> SigHandler {
		let table = self.table.lock();
		let handler = &table[num.index()];
		handler.clone()
	}

	pub fn is_default(&self, num: &SigNum) -> bool {
		self.table.lock()[num.index()].is_default()
	}
}

/// push data to user stack.
unsafe fn push_to_user_stack(esp: &mut usize, src: *const u8, len: usize) {
	*esp -= len;
	copy_nonoverlapping(src, (*esp) as *mut _, len);
}

fn make_function_frame(frame: &mut [usize], user_esp: usize, sig_num: SigNum) {
	let trampoline = trampoline_address(signal_trampoline as usize);

	frame[0] = trampoline;
	frame[1] = sig_num as usize;

	// SigInfo pointer
	frame[2] = user_esp;

	// SigCtx pointer
	frame[3] = user_esp + size_of::<SigInfo>()
}

/// # Safety
///
/// - CURRENT should be a user task.
pub unsafe fn poll_signal_queue() -> Result<(), Errno> {
	let signal = CURRENT
		.get_mut()
		.get_user_ext()
		.expect("user task")
		.signal
		.as_ref();
	let queue = signal.queue.lock();
	let mask = signal.mask.lock();

	let count = queue
		.iter()
		.filter(|info| !mask.contains(info.num.into()))
		.count();

	if count == 0 {
		Ok(())
	} else {
		// pr_debug!("poll_signal_queue: there is signal!");
		Err(Errno::EINTR)
	}
}

pub fn send_signal_to(task: &Arc<Task>, sig_info: &SigInfo) -> Result<(), Errno> {
	if task.get_pid().as_raw() == 1 {
		return Ok(()); // ignore signal
	}

	pr_debug!(
		"{:?} received SIG{:?} from pid[{}]",
		task.get_pid(),
		sig_info.num,
		sig_info.pid
	);

	task.recv_signal(sig_info.clone())?;
	wake_up(task, Sleep::Light);
	Ok(())
}

pub fn send_signal_to_foreground(
	sess: &Weak<Locked<Session>>,
	num: SigNum,
	code: SigCode,
) -> Result<(), Errno> {
	let sess = sess.upgrade().ok_or(Errno::EPERM)?;
	let sess_lock = sess.lock();
	let fg = sess_lock
		.foreground()
		.and_then(|w| w.upgrade())
		.ok_or(Errno::ESRCH)?;

	pr_debug!(
		"SIG{:?}: foreground: {:?}, {:?}",
		num,
		sess_lock.get_sid(),
		fg.get_pgid()
	);

	for (_, weak) in fg.lock_members().iter() {
		if let Some(task) = weak.upgrade() {
			let sig_info = SigInfo {
				num,
				pid: task.get_pid().as_raw(),
				uid: task.get_uid(),
				code,
			};
			let _ = send_signal_to(&task, &sig_info);
		}
	}
	Ok(())
}
