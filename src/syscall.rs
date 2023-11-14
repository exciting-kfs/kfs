pub mod clock;
pub mod errno;
pub mod exec;
pub mod fork;
pub mod kill;
pub mod relation;
pub mod sendfile;
pub mod signal;
pub mod wait;

mod dup;
mod reboot;
mod uname;

use core::fmt::{self, Display};
use core::mem::transmute;

use crate::driver::pipe::sys_pipe;
use crate::elf::syscall::*;
use crate::fs::syscall::*;
use crate::interrupt::InterruptFrame;
use crate::mm::user::brk::sys_brk;
use crate::mm::user::mmap::{sys_mmap, sys_munmap};

use crate::net::syscall::*;
use crate::process::exit::sys_exit;
use crate::process::gid::{sys_getgid, sys_setgid};
use crate::process::set_thread_area::{sys_set_thread_area, sys_set_tid_address};
use crate::process::signal::sig_handler::SigAction;
use crate::process::task::CURRENT;
use crate::process::uid::{sys_getuid, sys_setuid};
use crate::scheduler::nano_sleep::sys_nanosleep;
use crate::scheduler::sys_sched_yield;
use crate::{pr_warn, trace_feature};

use self::clock::sys_clock_gettime;
use self::dup::{sys_dup, sys_dup2};
use self::errno::Errno;
use self::exec::*;
use self::fork::sys_fork;
use self::kill::sys_kill;
use self::reboot::sys_reboot;
use self::relation::{
	sys_getpgid, sys_getpgrp, sys_getpid, sys_getppid, sys_getsid, sys_setpgid, sys_setsid,
};
use self::sendfile::sys_sendfile;
use self::signal::{sys_sigaction, sys_signal, sys_sigprocmask, sys_sigreturn, sys_sigsuspend};
use self::uname::sys_uname;
use self::wait::sys_waitpid;

/// `syscall no` must be sorted.
const IGNORE_SYSCALL_RESTART: [usize; 2] = [162, 179];

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let signal = unsafe {
		CURRENT
			.get_mut()
			.get_user_ext()
			.expect("user task")
			.signal
			.as_ref()
	};

	let mut ret;
	loop {
		let mut restart = false;
		ret = syscall(&mut frame, &mut restart);

		if matches!(
			signal.do_signal(&frame, syscall_return_to_isize(&ret)),
			Some(_)
		) || restart
		{
			continue;
		}

		break;
	}

	// Because of signal system, This can be `return` from timer interrupt.
	// To preserve previous `eax` value, We should check where this `return` go.
	if frame.handler == handle_syscall_impl as usize {
		unsafe {
			((&mut frame.eax) as *mut _ as usize as *mut isize)
				.write_volatile(syscall_return_to_isize(&ret))
		};
	}
}

fn get_syscall_info(n: usize) -> (&'static str, usize) {
	match n {
		0 => ("restart_syscall", 0),
		1 => ("exit", 1),
		2 => ("fork", 0),
		3 => ("read", 3),
		4 => ("write", 3),
		5 => ("open", 3),
		6 => ("close", 1),
		7 => ("waitpid", 3),
		8 => ("creat", 2),
		9 => ("link", 2),
		10 => ("unlink", 1),
		11 => ("execve", 3),
		12 => ("chdir", 1),
		13 => ("time", 1),
		14 => ("mknod", 3),
		15 => ("chmod", 2),
		16 => ("lchown", 3),
		17 => ("break", 6),
		18 => ("oldstat", 6),
		19 => ("lseek", 3),
		20 => ("getpid", 0),
		21 => ("mount", 5),
		22 => ("umount", 2),
		23 => ("setuid", 1),
		24 => ("getuid", 0),
		25 => ("stime", 1),
		26 => ("ptrace", 4),
		27 => ("alarm", 1),
		28 => ("oldfstat", 6),
		29 => ("pause", 0),
		30 => ("utime", 2),
		31 => ("stty", 6),
		32 => ("gtty", 6),
		33 => ("access", 2),
		34 => ("nice", 1),
		35 => ("ftime", 6),
		36 => ("sync", 0),
		37 => ("kill", 2),
		38 => ("rename", 2),
		39 => ("mkdir", 2),
		40 => ("rmdir", 1),
		41 => ("dup", 1),
		42 => ("pipe", 1),
		43 => ("times", 1),
		44 => ("prof", 6),
		45 => ("brk", 1),
		46 => ("setgid", 1),
		47 => ("getgid", 0),
		48 => ("signal", 2),
		49 => ("geteuid", 0),
		50 => ("getegid", 0),
		51 => ("acct", 1),
		52 => ("umount2", 2),
		53 => ("lock", 6),
		54 => ("ioctl", 3),
		55 => ("fcntl", 3),
		56 => ("mpx", 6),
		57 => ("setpgid", 2),
		58 => ("ulimit", 6),
		59 => ("oldolduname", 6),
		60 => ("umask", 1),
		61 => ("chroot", 1),
		62 => ("ustat", 2),
		63 => ("dup2", 2),
		64 => ("getppid", 0),
		65 => ("getpgrp", 0),
		66 => ("setsid", 0),
		67 => ("sigaction", 3),
		68 => ("sgetmask", 0),
		69 => ("ssetmask", 1),
		70 => ("setreuid", 2),
		71 => ("setregid", 2),
		72 => ("sigsuspend", 3),
		73 => ("sigpending", 1),
		74 => ("sethostname", 2),
		75 => ("setrlimit", 2),
		76 => ("getrlimit", 2),
		77 => ("getrusage", 2),
		78 => ("gettimeofday", 2),
		79 => ("settimeofday", 2),
		80 => ("getgroups", 2),
		81 => ("setgroups", 2),
		82 => ("select", 5),
		83 => ("symlink", 2),
		84 => ("oldlstat", 6),
		85 => ("readlink", 3),
		86 => ("uselib", 1),
		87 => ("swapon", 2),
		88 => ("reboot", 4),
		89 => ("readdir", 6),
		90 => ("mmap", 6),
		91 => ("munmap", 2),
		92 => ("truncate", 2),
		93 => ("ftruncate", 2),
		94 => ("fchmod", 2),
		95 => ("fchown", 3),
		96 => ("getpriority", 2),
		97 => ("setpriority", 3),
		98 => ("profil", 6),
		99 => ("statfs", 2),
		100 => ("fstatfs", 2),
		101 => ("ioperm", 3),
		102 => ("socketcall", 2),
		103 => ("syslog", 3),
		104 => ("setitimer", 3),
		105 => ("getitimer", 2),
		106 => ("stat", 2),
		107 => ("lstat", 2),
		108 => ("fstat", 2),
		109 => ("olduname", 1),
		110 => ("iopl", 6),
		111 => ("vhangup", 0),
		112 => ("idle", 6),
		113 => ("vm86old", 6),
		114 => ("wait4", 4),
		115 => ("swapoff", 1),
		116 => ("sysinfo", 1),
		117 => ("ipc", 6),
		118 => ("fsync", 1),
		119 => ("sigreturn", 6),
		120 => ("clone", 5),
		121 => ("setdomainname", 2),
		122 => ("uname", 1),
		123 => ("modify_ldt", 6),
		124 => ("adjtimex", 1),
		125 => ("mprotect", 3),
		126 => ("sigprocmask", 3),
		127 => ("create_module", 6),
		128 => ("init_module", 3),
		129 => ("delete_module", 2),
		130 => ("get_kernel_syms", 6),
		131 => ("quotactl", 4),
		132 => ("getpgid", 1),
		133 => ("fchdir", 1),
		134 => ("bdflush", 2),
		135 => ("sysfs", 3),
		136 => ("personality", 1),
		137 => ("afs_syscall", 6),
		138 => ("setfsuid", 1),
		139 => ("setfsgid", 1),
		140 => ("_llseek", 6),
		141 => ("getdents", 3),
		142 => ("_newselect", 6),
		143 => ("flock", 2),
		144 => ("msync", 3),
		145 => ("readv", 3),
		146 => ("writev", 3),
		147 => ("getsid", 1),
		148 => ("fdatasync", 1),
		149 => ("_sysctl", 6),
		150 => ("mlock", 2),
		151 => ("munlock", 2),
		152 => ("mlockall", 1),
		153 => ("munlockall", 0),
		154 => ("sched_setparam", 2),
		155 => ("sched_getparam", 2),
		156 => ("sched_setscheduler", 3),
		157 => ("sched_getscheduler", 1),
		158 => ("sched_yield", 0),
		159 => ("sched_get_priority_max", 1),
		160 => ("sched_get_priority_min", 1),
		161 => ("sched_rr_get_interval", 2),
		162 => ("nanosleep", 2),
		163 => ("mremap", 5),
		164 => ("setresuid", 3),
		165 => ("getresuid", 3),
		166 => ("vm86", 6),
		167 => ("query_module", 6),
		168 => ("poll", 3),
		169 => ("nfsservctl", 6),
		170 => ("setresgid", 3),
		171 => ("getresgid", 3),
		172 => ("prctl", 5),
		173 => ("rt_sigreturn", 6),
		174 => ("rt_sigaction", 4),
		175 => ("rt_sigprocmask", 4),
		176 => ("rt_sigpending", 2),
		177 => ("rt_sigtimedwait", 4),
		178 => ("rt_sigqueueinfo", 3),
		179 => ("rt_sigsuspend", 2),
		180 => ("pread64", 4),
		181 => ("pwrite64", 4),
		182 => ("chown", 3),
		183 => ("getcwd", 2),
		184 => ("capget", 2),
		185 => ("capset", 2),
		186 => ("sigaltstack", 2),
		187 => ("sendfile", 4),
		188 => ("getpmsg", 6),
		189 => ("putpmsg", 6),
		190 => ("vfork", 0),
		191 => ("ugetrlimit", 6),
		192 => ("mmap2", 6),
		193 => ("truncate64", 2),
		194 => ("ftruncate64", 2),
		195 => ("stat64", 2),
		196 => ("lstat64", 2),
		197 => ("fstat64", 2),
		198 => ("lchown32", 6),
		199 => ("getuid32", 6),
		200 => ("getgid32", 6),
		201 => ("geteuid32", 6),
		202 => ("getegid32", 6),
		203 => ("setreuid32", 6),
		204 => ("setregid32", 6),
		205 => ("getgroups32", 6),
		206 => ("setgroups32", 6),
		207 => ("fchown32", 6),
		208 => ("setresuid32", 6),
		209 => ("getresuid32", 6),
		210 => ("setresgid32", 6),
		211 => ("getresgid32", 6),
		212 => ("chown32", 6),
		213 => ("setuid32", 6),
		214 => ("setgid32", 6),
		215 => ("setfsuid32", 6),
		216 => ("setfsgid32", 6),
		217 => ("pivot_root", 2),
		218 => ("mincore", 3),
		219 => ("madvise", 3),
		220 => ("getdents64", 3),
		221 => ("fcntl64", 3),
		224 => ("gettid", 0),
		225 => ("readahead", 3),
		226 => ("setxattr", 5),
		227 => ("lsetxattr", 5),
		228 => ("fsetxattr", 5),
		229 => ("getxattr", 4),
		230 => ("lgetxattr", 4),
		231 => ("fgetxattr", 4),
		232 => ("listxattr", 3),
		233 => ("llistxattr", 3),
		234 => ("flistxattr", 3),
		235 => ("removexattr", 2),
		236 => ("lremovexattr", 2),
		237 => ("fremovexattr", 2),
		238 => ("tkill", 2),
		239 => ("sendfile64", 4),
		240 => ("futex", 6),
		241 => ("sched_setaffinity", 3),
		242 => ("sched_getaffinity", 3),
		243 => ("set_thread_area", 6),
		244 => ("get_thread_area", 6),
		245 => ("io_setup", 2),
		246 => ("io_destroy", 1),
		247 => ("io_getevents", 5),
		248 => ("io_submit", 3),
		249 => ("io_cancel", 3),
		250 => ("fadvise64", 4),
		252 => ("exit_group", 1),
		253 => ("lookup_dcookie", 3),
		254 => ("epoll_create", 1),
		255 => ("epoll_ctl", 4),
		256 => ("epoll_wait", 4),
		257 => ("remap_file_pages", 5),
		258 => ("set_tid_address", 1),
		259 => ("timer_create", 3),
		260 => ("timer_settime", 4),
		261 => ("timer_gettime", 2),
		262 => ("timer_getoverrun", 1),
		263 => ("timer_delete", 1),
		264 => ("clock_settime", 2),
		265 => ("clock_gettime", 2),
		266 => ("clock_getres", 2),
		267 => ("clock_nanosleep", 4),
		268 => ("statfs64", 3),
		269 => ("fstatfs64", 3),
		270 => ("tgkill", 3),
		271 => ("utimes", 2),
		272 => ("fadvise64_64", 4),
		273 => ("vserver", 6),
		274 => ("mbind", 6),
		275 => ("get_mempolicy", 5),
		276 => ("set_mempolicy", 3),
		277 => ("mq_open", 4),
		278 => ("mq_unlink", 1),
		279 => ("mq_timedsend", 5),
		280 => ("mq_timedreceive", 5),
		281 => ("mq_notify", 2),
		282 => ("mq_getsetattr", 3),
		283 => ("kexec_load", 4),
		284 => ("waitid", 5),
		286 => ("add_key", 5),
		287 => ("request_key", 4),
		288 => ("keyctl", 5),
		289 => ("ioprio_set", 3),
		290 => ("ioprio_get", 2),
		291 => ("inotify_init", 0),
		292 => ("inotify_add_watch", 3),
		293 => ("inotify_rm_watch", 2),
		294 => ("migrate_pages", 4),
		295 => ("openat", 4),
		296 => ("mkdirat", 3),
		297 => ("mknodat", 4),
		298 => ("fchownat", 5),
		299 => ("futimesat", 3),
		300 => ("fstatat64", 4),
		301 => ("unlinkat", 3),
		302 => ("renameat", 4),
		303 => ("linkat", 5),
		304 => ("symlinkat", 3),
		305 => ("readlinkat", 4),
		306 => ("fchmodat", 3),
		307 => ("faccessat", 3),
		308 => ("pselect6", 6),
		309 => ("ppoll", 5),
		310 => ("unshare", 1),
		311 => ("set_robust_list", 2),
		312 => ("get_robust_list", 3),
		313 => ("splice", 6),
		314 => ("sync_file_range", 4),
		315 => ("tee", 4),
		316 => ("vmsplice", 4),
		317 => ("move_pages", 6),
		318 => ("getcpu", 3),
		319 => ("epoll_pwait", 6),
		320 => ("utimensat", 4),
		321 => ("signalfd", 3),
		322 => ("timerfd_create", 2),
		323 => ("eventfd", 1),
		324 => ("fallocate", 4),
		325 => ("timerfd_settime", 4),
		326 => ("timerfd_gettime", 2),
		327 => ("signalfd4", 4),
		328 => ("eventfd2", 2),
		329 => ("epoll_create1", 1),
		330 => ("dup3", 3),
		331 => ("pipe2", 2),
		332 => ("inotify_init1", 1),
		333 => ("preadv", 5),
		334 => ("pwritev", 5),
		335 => ("rt_tgsigqueueinfo", 4),
		336 => ("perf_event_open", 5),
		337 => ("recvmmsg", 5),
		338 => ("fanotify_init", 2),
		339 => ("fanotify_mark", 5),
		340 => ("prlimit64", 4),
		341 => ("name_to_handle_at", 5),
		342 => ("open_by_handle_at", 3),
		343 => ("clock_adjtime", 2),
		344 => ("syncfs", 1),
		345 => ("sendmmsg", 4),
		346 => ("setns", 2),
		347 => ("process_vm_readv", 6),
		348 => ("process_vm_writev", 6),
		349 => ("kcmp", 5),
		350 => ("finit_module", 3),
		351 => ("sched_setattr", 3),
		352 => ("sched_getattr", 4),
		353 => ("renameat2", 5),
		354 => ("seccomp", 3),
		355 => ("getrandom", 3),
		356 => ("memfd_create", 2),
		357 => ("bpf", 3),
		358 => ("execveat", 5),
		359 => ("socket", 3),
		360 => ("socketpair", 4),
		361 => ("bind", 3),
		362 => ("connect", 3),
		363 => ("listen", 2),
		364 => ("accept4", 4),
		365 => ("getsockopt", 5),
		366 => ("setsockopt", 5),
		367 => ("getsockname", 3),
		368 => ("getpeername", 3),
		369 => ("sendto", 6),
		370 => ("sendmsg", 3),
		371 => ("recvfrom", 6),
		372 => ("recvmsg", 3),
		373 => ("shutdown", 2),
		374 => ("userfaultfd", 1),
		375 => ("membarrier", 2),
		376 => ("mlock2", 3),
		377 => ("copy_file_range", 6),
		378 => ("preadv2", 6),
		379 => ("pwritev2", 6),
		380 => ("pkey_mprotect", 4),
		381 => ("pkey_alloc", 2),
		382 => ("pkey_free", 1),
		383 => ("statx", 5),
		384 => ("arch_prctl", 6),
		403 => ("clock_gettime64", 2),
		_ => ("unknown", 6),
	}
}

struct SyscallSnapshot {
	nr: usize,
	name: &'static str,
	args: [usize; 6],
	arg_count: usize,
}

impl SyscallSnapshot {
	pub fn new(frame: &InterruptFrame) -> Self {
		let args = [
			frame.ebx, frame.ecx, frame.edx, frame.esi, frame.edi, frame.ebp,
		];

		let nr = frame.eax;
		let (name, arg_count) = get_syscall_info(nr);

		Self {
			nr,
			name,
			args,
			arg_count,
		}
	}
}

impl Display for SyscallSnapshot {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}(no={}) args:\n", self.name, self.nr)?;

		for (i, arg) in self.args.iter().take(self.arg_count).enumerate() {
			write!(f, " #{}: {:#010x}\n", i + 1, arg)?;
		}

		Ok(())
	}
}

fn syscall(frame: &mut InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	if cfg!(trace_feature = "syscall") {
		let ret = __syscall(frame, restart);
		trace_feature!(
			"syscall",
			"{:?}: {} #R: {:?}",
			unsafe { CURRENT.get_ref().get_pid() },
			SyscallSnapshot::new(frame),
			ret
		);
		ret
	} else {
		__syscall(frame, restart)
	}
}

fn __syscall(frame: &mut InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	match frame.eax {
		1 => sys_exit(frame.ebx),
		2 => sys_fork(frame),
		3 => sys_read(frame.ebx as isize, frame.ecx, frame.edx),
		4 => sys_write(frame.ebx as isize, frame.ecx, frame.edx),
		5 => sys_open(frame.ebx, frame.ecx as i32, frame.edx as u32),
		6 => sys_close(frame.ebx as isize),
		7 => sys_waitpid(frame.ebx as isize, frame.ecx as *mut isize, frame.edx),
		8 => sys_creat(frame.ebx, frame.ecx as u32),
		9 => sys_link(frame.ebx, frame.ecx),
		10 => sys_unlink(frame.ebx),
		11 => sys_execve(frame, frame.ebx, frame.ecx, frame.edx),
		12 => sys_chdir(frame.ebx),
		15 => sys_chmod(frame.ebx, frame.ecx as u32),
		19 => sys_lseek(frame.ebx as isize, frame.ecx as isize, frame.edx as isize),
		20 => sys_getpid(),
		21 => sys_mount(frame.ebx, frame.ecx, frame.edx),
		22 => sys_umount(frame.ebx),
		37 => sys_kill(frame.ebx as isize, frame.ecx as isize),
		38 => sys_rename(frame.ebx, frame.ecx),
		39 => sys_mkdir(frame.ebx, frame.ecx as u32),
		40 => sys_rmdir(frame.ebx),
		41 => sys_dup(frame.ebx),
		42 => sys_pipe(frame.ebx),
		45 => sys_brk(frame.ebx),
		48 => sys_signal(frame.ebx, frame.ecx),
		// todo: umount2
		52 => sys_umount(frame.ebx),
		54 => sys_ioctl(frame.ebx as isize, frame.ecx, frame.edx),
		55 | 221 => sys_fcntl(frame.ebx as isize, frame.ecx, frame.edx),
		57 => sys_setpgid(frame.ebx, frame.ecx),
		63 => sys_dup2(frame.ebx, frame.ecx),
		64 => sys_getppid(),
		65 => sys_getpgrp(),
		66 => sys_setsid(),
		// sigaction / rt_sigaction
		67 | 174 => sys_sigaction(
			frame.ebx,
			frame.ecx as *const SigAction,
			frame.edx as *mut SigAction,
		),
		80 => sys_reboot(frame.ebx),
		83 => sys_symlink(frame.ebx, frame.ecx),
		85 => sys_readlink(frame.ebx, frame.ecx, frame.edx),
		// mmap / mmap2 TODO: proper mmap2 handling
		90 | 192 => sys_mmap(
			frame.ebx,
			frame.ecx,
			frame.edx as i32,
			frame.esi as i32,
			frame.edi as i32,
			frame.ebp as isize,
		)
		.map_err(|_| Errno::EPERM), // FIXME: proper return type
		91 => sys_munmap(frame.ebx, frame.ecx),
		92 => sys_truncate(frame.ebx, frame.ecx as isize),
		// TODO: wait4
		114 => sys_waitpid(frame.ebx as isize, frame.ecx as *mut isize, frame.edx),
		119 => sys_sigreturn(frame, restart),
		122 => sys_uname(frame.ebx),
		128 => sys_init_module(frame.ebx),
		129 => sys_cleanup_module(frame.ebx),
		132 => sys_getpgid(frame.ebx),
		141 => sys_getdents(frame.ebx as isize, frame.ecx, frame.edx),
		146 => sys_writev(frame.ebx as isize, frame.ecx, frame.edx),
		147 => sys_getsid(frame.ebx),
		158 => sys_sched_yield(),
		162 => sys_nanosleep(frame.ebx, frame.ecx),
		// poll
		168 => Ok(frame.ecx),
		// TODO: rt_sigprocmask
		175 => sys_sigprocmask(frame.ebx, frame.ecx, frame.edx),
		// TODO: rt_sigsuspend
		179 => sys_sigsuspend(frame.ebx),
		183 => sys_getcwd(frame.ebx, frame.ecx),
		199 => sys_getuid(),
		200 => sys_getgid(),
		212 => sys_chown(frame.ebx, frame.ecx, frame.edx),
		213 => sys_setuid(frame.ebx),
		214 => sys_setgid(frame.ebx),
		220 => sys_getdents(frame.ebx as isize, frame.ecx, frame.edx),
		239 => sys_sendfile(frame.ebx as isize, frame.ecx as isize, frame.edx, frame.esi),
		243 => sys_set_thread_area(frame.ebx),
		// TODO: exit_group
		252 => sys_exit(frame.ebx),
		// TODO: set_tid_address
		258 => sys_set_tid_address(frame.ebx),
		265 => sys_clock_gettime(frame.ebx, frame.ecx),
		268 => sys_statfs64(frame.ebx, frame.ecx, frame.edx),
		359 => sys_socket(frame.ebx as i32, frame.ecx as i32, frame.edx as i32),
		361 => sys_bind(frame.ebx, frame.ecx, frame.edx),
		362 => sys_connect(frame.ebx, frame.ecx, frame.edx),
		363 => sys_listen(frame.ebx, frame.ecx),
		364 => sys_accept(frame.ebx, frame.ecx, frame.edx),
		369 => sys_sendto(
			frame.ebx as isize,
			frame.ecx,
			frame.edx,
			frame.esi,
			frame.edi,
		),
		371 => sys_recvfrom(
			frame.ebx as isize,
			frame.ecx,
			frame.edx,
			frame.esi,
			frame.edi,
		),
		// vfork
		190 => sys_fork(frame),
		// tkill
		238 => sys_kill(frame.ebx as isize, frame.ecx as isize),
		320 => sys_utimensat(frame.ebx as isize, frame.ecx, frame.edx, frame.esi),
		// statx
		383 => sys_statx(
			frame.ebx as isize,
			frame.ecx,
			frame.edx,
			frame.esi,
			frame.edi,
		),
		// clock_gettime64
		403 => Err(Errno::ENOSYS),
		_ => {
			pr_warn!(
				"unimplemented syscall: {}(no={})",
				get_syscall_info(frame.eax).0,
				frame.eax,
			);
			Ok(0)
		}
	}
}

pub fn syscall_return_to_isize(result: &Result<usize, Errno>) -> isize {
	match result {
		Ok(u) => *u as isize,
		Err(e) => e.as_ret(),
	}
}

pub fn restore_syscall_return(result: isize) -> Result<usize, Errno> {
	if result < 0 {
		Err(unsafe { transmute(-result) })
	} else {
		Ok(result as usize)
	}
}
