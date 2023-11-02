pub mod errno;
pub mod exec;
pub mod fork;
pub mod kill;
pub mod relation;
pub mod signal;
pub mod wait;

mod reboot;

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
use crate::process::set_thread_area::sys_set_thread_area;
use crate::process::signal::sig_handler::SigAction;
use crate::process::task::CURRENT;
use crate::process::uid::{sys_getuid, sys_setuid};
use crate::scheduler::sys_sched_yield;
use crate::{pr_info, trace_feature};

use self::errno::Errno;
use self::exec::*;
use self::fork::sys_fork;
use self::kill::sys_kill;
use self::reboot::sys_reboot;
use self::relation::{
	sys_getpgid, sys_getpgrp, sys_getpid, sys_getppid, sys_getsid, sys_setpgid, sys_setsid,
};
use self::signal::{sys_sigaction, sys_signal, sys_sigreturn};
use self::wait::sys_waitpid;

#[no_mangle]
pub extern "C" fn handle_syscall_impl(mut frame: InterruptFrame) {
	let mut restart = true;
	let mut ret = Err(Errno::UnknownErrno);
	let signal = unsafe {
		CURRENT
			.get_mut()
			.get_user_ext()
			.expect("user task")
			.signal
			.as_ref()
	};

	while restart {
		restart = false;
		ret = syscall(&mut frame, &mut restart);

		if let Some(_) = signal.do_signal(&frame, syscall_return_to_isize(&ret)) {
			restart = true;
		}
		// use crate::pr_debug;
		// pr_debug!("syscall: ret: {:?}", ret);
		// pr_debug!("syscall: restart: {}", restart);
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

fn get_syscall_name(n: usize) -> &'static str {
	match n {
		0 => "restart_syscall",
		1 => "exit",
		2 => "fork",
		3 => "read",
		4 => "write",
		5 => "open",
		6 => "close",
		7 => "waitpid",
		8 => "creat",
		9 => "link",
		10 => "unlink",
		11 => "execve",
		12 => "chdir",
		13 => "time",
		14 => "mknod",
		15 => "chmod",
		16 => "lchown",
		17 => "break",
		18 => "oldstat",
		19 => "lseek",
		20 => "getpid",
		21 => "mount",
		22 => "umount",
		23 => "setuid",
		24 => "getuid",
		25 => "stime",
		26 => "ptrace",
		27 => "alarm",
		28 => "oldfstat",
		29 => "pause",
		30 => "utime",
		31 => "stty",
		32 => "gtty",
		33 => "access",
		34 => "nice",
		35 => "ftime",
		36 => "sync",
		37 => "kill",
		38 => "rename",
		39 => "mkdir",
		40 => "rmdir",
		41 => "dup",
		42 => "pipe",
		43 => "times",
		44 => "prof",
		45 => "brk",
		46 => "setgid",
		47 => "getgid",
		48 => "signal",
		49 => "geteuid",
		50 => "getegid",
		51 => "acct",
		52 => "umount2",
		53 => "lock",
		54 => "ioctl",
		55 => "fcntl",
		56 => "mpx",
		57 => "setpgid",
		58 => "ulimit",
		59 => "oldolduname",
		60 => "umask",
		61 => "chroot",
		62 => "ustat",
		63 => "dup2",
		64 => "getppid",
		65 => "getpgrp",
		66 => "setsid",
		67 => "sigaction",
		68 => "sgetmask",
		69 => "ssetmask",
		70 => "setreuid",
		71 => "setregid",
		72 => "sigsuspend",
		73 => "sigpending",
		74 => "sethostname",
		75 => "setrlimit",
		76 => "getrlimit",
		77 => "getrusage",
		78 => "gettimeofday",
		79 => "settimeofday",
		80 => "getgroups",
		81 => "setgroups",
		82 => "select",
		83 => "symlink",
		84 => "oldlstat",
		85 => "readlink",
		86 => "uselib",
		87 => "swapon",
		88 => "reboot",
		89 => "readdir",
		90 => "mmap",
		91 => "munmap",
		92 => "truncate",
		93 => "ftruncate",
		94 => "fchmod",
		95 => "fchown",
		96 => "getpriority",
		97 => "setpriority",
		98 => "profil",
		99 => "statfs",
		100 => "fstatfs",
		101 => "ioperm",
		102 => "socketcall",
		103 => "syslog",
		104 => "setitimer",
		105 => "getitimer",
		106 => "stat",
		107 => "lstat",
		108 => "fstat",
		109 => "olduname",
		110 => "iopl",
		111 => "vhangup",
		112 => "idle",
		113 => "vm86old",
		114 => "wait4",
		115 => "swapoff",
		116 => "sysinfo",
		117 => "ipc",
		118 => "fsync",
		119 => "sigreturn",
		120 => "clone",
		121 => "setdomainname",
		122 => "uname",
		123 => "modify_ldt",
		124 => "adjtimex",
		125 => "mprotect",
		126 => "sigprocmask",
		127 => "create_module",
		128 => "init_module",
		129 => "delete_module",
		130 => "get_kernel_syms",
		131 => "quotactl",
		132 => "getpgid",
		133 => "fchdir",
		134 => "bdflush",
		135 => "sysfs",
		136 => "personality",
		137 => "afs_syscall",
		138 => "setfsuid",
		139 => "setfsgid",
		140 => "_llseek",
		141 => "getdents",
		142 => "_newselect",
		143 => "flock",
		144 => "msync",
		145 => "readv",
		146 => "writev",
		147 => "getsid",
		148 => "fdatasync",
		149 => "_sysctl",
		150 => "mlock",
		151 => "munlock",
		152 => "mlockall",
		153 => "munlockall",
		154 => "sched_setparam",
		155 => "sched_getparam",
		156 => "sched_setscheduler",
		157 => "sched_getscheduler",
		158 => "sched_yield",
		159 => "sched_get_priority_max",
		160 => "sched_get_priority_min",
		161 => "sched_rr_get_interval",
		162 => "nanosleep",
		163 => "mremap",
		164 => "setresuid",
		165 => "getresuid",
		166 => "vm86",
		167 => "query_module",
		168 => "poll",
		169 => "nfsservctl",
		170 => "setresgid",
		171 => "getresgid",
		172 => "prctl",
		173 => "rt_sigreturn",
		174 => "rt_sigaction",
		175 => "rt_sigprocmask",
		176 => "rt_sigpending",
		177 => "rt_sigtimedwait",
		178 => "rt_sigqueueinfo",
		179 => "rt_sigsuspend",
		180 => "pread64",
		181 => "pwrite64",
		182 => "chown",
		183 => "getcwd",
		184 => "capget",
		185 => "capset",
		186 => "sigaltstack",
		187 => "sendfile",
		188 => "getpmsg",
		189 => "putpmsg",
		190 => "vfork",
		191 => "ugetrlimit",
		192 => "mmap2",
		193 => "truncate64",
		194 => "ftruncate64",
		195 => "stat64",
		196 => "lstat64",
		197 => "fstat64",
		198 => "lchown32",
		199 => "getuid32",
		200 => "getgid32",
		201 => "geteuid32",
		202 => "getegid32",
		203 => "setreuid32",
		204 => "setregid32",
		205 => "getgroups32",
		206 => "setgroups32",
		207 => "fchown32",
		208 => "setresuid32",
		209 => "getresuid32",
		210 => "setresgid32",
		211 => "getresgid32",
		212 => "chown32",
		213 => "setuid32",
		214 => "setgid32",
		215 => "setfsuid32",
		216 => "setfsgid32",
		217 => "pivot_root",
		218 => "mincore",
		219 => "madvise",
		220 => "getdents64",
		221 => "fcntl64",
		222 => "not implemented",
		223 => "not implemented",
		224 => "gettid",
		225 => "readahead",
		226 => "setxattr",
		227 => "lsetxattr",
		228 => "fsetxattr",
		229 => "getxattr",
		230 => "lgetxattr",
		231 => "fgetxattr",
		232 => "listxattr",
		233 => "llistxattr",
		234 => "flistxattr",
		235 => "removexattr",
		236 => "lremovexattr",
		237 => "fremovexattr",
		238 => "tkill",
		239 => "sendfile64",
		240 => "futex",
		241 => "sched_setaffinity",
		242 => "sched_getaffinity",
		243 => "set_thread_area",
		244 => "get_thread_area",
		245 => "io_setup",
		246 => "io_destroy",
		247 => "io_getevents",
		248 => "io_submit",
		249 => "io_cancel",
		250 => "fadvise64",
		251 => "not implemented",
		252 => "exit_group",
		253 => "lookup_dcookie",
		254 => "epoll_create",
		255 => "epoll_ctl",
		256 => "epoll_wait",
		257 => "remap_file_pages",
		258 => "set_tid_address",
		259 => "timer_create",
		260 => "timer_settime",
		261 => "timer_gettime",
		262 => "timer_getoverrun",
		263 => "timer_delete",
		264 => "clock_settime",
		265 => "clock_gettime",
		266 => "clock_getres",
		267 => "clock_nanosleep",
		268 => "statfs64",
		269 => "fstatfs64",
		270 => "tgkill",
		271 => "utimes",
		272 => "fadvise64_64",
		273 => "vserver",
		274 => "mbind",
		275 => "get_mempolicy",
		276 => "set_mempolicy",
		277 => "mq_open",
		278 => "mq_unlink",
		279 => "mq_timedsend",
		280 => "mq_timedreceive",
		281 => "mq_notify",
		282 => "mq_getsetattr",
		283 => "kexec_load",
		284 => "waitid",
		285 => "not implemented",
		286 => "add_key",
		287 => "request_key",
		288 => "keyctl",
		289 => "ioprio_set",
		290 => "ioprio_get",
		291 => "inotify_init",
		292 => "inotify_add_watch",
		293 => "inotify_rm_watch",
		294 => "migrate_pages",
		295 => "openat",
		296 => "mkdirat",
		297 => "mknodat",
		298 => "fchownat",
		299 => "futimesat",
		300 => "fstatat64",
		301 => "unlinkat",
		302 => "renameat",
		303 => "linkat",
		304 => "symlinkat",
		305 => "readlinkat",
		306 => "fchmodat",
		307 => "faccessat",
		308 => "pselect6",
		309 => "ppoll",
		310 => "unshare",
		311 => "set_robust_list",
		312 => "get_robust_list",
		313 => "splice",
		314 => "sync_file_range",
		315 => "tee",
		316 => "vmsplice",
		317 => "move_pages",
		318 => "getcpu",
		319 => "epoll_pwait",
		320 => "utimensat",
		321 => "signalfd",
		322 => "timerfd_create",
		323 => "eventfd",
		324 => "fallocate",
		325 => "timerfd_settime",
		326 => "timerfd_gettime",
		327 => "signalfd4",
		328 => "eventfd2",
		329 => "epoll_create1",
		330 => "dup3",
		331 => "pipe2",
		332 => "inotify_init1",
		333 => "preadv",
		334 => "pwritev",
		335 => "rt_tgsigqueueinfo",
		336 => "perf_event_open",
		337 => "recvmmsg",
		338 => "fanotify_init",
		339 => "fanotify_mark",
		340 => "prlimit64",
		341 => "name_to_handle_at",
		342 => "open_by_handle_at",
		343 => "clock_adjtime",
		344 => "syncfs",
		345 => "sendmmsg",
		346 => "setns",
		347 => "process_vm_readv",
		348 => "process_vm_writev",
		349 => "kcmp",
		350 => "finit_module",
		351 => "sched_setattr",
		352 => "sched_getattr",
		353 => "renameat2",
		354 => "seccomp",
		355 => "getrandom",
		356 => "memfd_create",
		357 => "bpf",
		358 => "execveat",
		359 => "socket",
		360 => "socketpair",
		361 => "bind",
		362 => "connect",
		363 => "listen",
		364 => "accept4",
		365 => "getsockopt",
		366 => "setsockopt",
		367 => "getsockname",
		368 => "getpeername",
		369 => "sendto",
		370 => "sendmsg",
		371 => "recvfrom",
		372 => "recvmsg",
		373 => "shutdown",
		374 => "userfaultfd",
		375 => "membarrier",
		376 => "mlock2",
		377 => "copy_file_range",
		378 => "preadv2",
		379 => "pwritev2",
		380 => "pkey_mprotect",
		381 => "pkey_alloc",
		382 => "pkey_free",
		383 => "statx",
		384 => "arch_prctl",
		_ => "unknown",
	}
}

fn syscall(frame: &mut InterruptFrame, restart: &mut bool) -> Result<usize, Errno> {
	match frame.eax {
		1 => {
			// pr_info!("PID[{}]: exited({})", current.get_pid().as_raw(), frame.ebx);
			sys_exit(frame.ebx);
		}
		2 => sys_fork(frame),
		3 => {
			// pr_debug!("syscall: read");
			sys_read(frame.ebx as isize, frame.ecx, frame.edx)
		}
		4 => {
			// pr_debug!("syscall: write");
			sys_write(frame.ebx as isize, frame.ecx, frame.edx)
		}
		5 => sys_open(frame.ebx, frame.ecx as i32, frame.edx as u32),
		6 => sys_close(frame.ebx as isize),
		7 => sys_waitpid(frame.ebx as isize, frame.ecx as *mut isize, frame.edx),
		8 => sys_creat(frame.ebx, frame.ecx as u32),
		10 => sys_unlink(frame.ebx),
		11 => sys_execve(frame, frame.ebx, frame.ecx, frame.edx),
		12 => sys_chdir(frame.ebx),
		15 => sys_chmod(frame.ebx, frame.ecx as u32),
		18 => sys_stat(frame.ebx, frame.ecx),
		19 => sys_lseek(frame.ebx as isize, frame.ecx as isize, frame.edx as isize),
		20 => sys_getpid(),
		21 => sys_mount(frame.ebx, frame.ecx, frame.edx),
		22 => sys_umount(frame.ebx),
		37 => sys_kill(frame.ebx as isize, frame.ecx as isize),
		39 => sys_mkdir(frame.ebx, frame.ecx as u32),
		40 => sys_rmdir(frame.ebx),
		42 => sys_pipe(frame.ebx),
		45 => sys_brk(frame.ebx),
		48 => {
			pr_info!("syscall: signal: {}, {:x}", frame.ebx, frame.ecx);
			sys_signal(frame.ebx, frame.ecx)
		}
		57 => sys_setpgid(frame.ebx, frame.ecx),
		65 => sys_getppid(),
		64 => sys_getpgrp(),
		66 => sys_setsid(),
		67 => {
			// pr_info!(
			// 	"syscall: sigaction: {}, {:x}, {:x}",
			// 	frame.ebx,
			// 	frame.ecx,
			// 	frame.edx
			// );
			sys_sigaction(
				frame.ebx,
				frame.ecx as *const SigAction,
				frame.edx as *mut SigAction,
			)
		}
		80 => sys_reboot(frame.ebx),
		83 => sys_symlink(frame.ebx, frame.ecx),
		90 => sys_mmap(
			frame.ebx,
			frame.ecx,
			frame.edx as i32,
			frame.esi as i32,
			frame.edi as i32,
			frame.ebp as isize,
		)
		.map_err(|_| Errno::UnknownErrno), // FIXME: proper return type
		91 => sys_munmap(frame.ebx, frame.ecx),
		92 => sys_truncate(frame.ebx, frame.ecx as isize),
		119 => {
			// pr_info!("syscall: sigreturn: {:p}", &frame);
			sys_sigreturn(frame, restart)
		}
		128 => sys_init_module(frame.ebx),
		129 => sys_cleanup_module(frame.ebx),
		132 => sys_getpgid(frame.ebx),
		141 => sys_getdents(frame.ebx as isize, frame.ecx, frame.edx),
		146 => sys_writev(frame.ebx as isize, frame.ecx, frame.edx),
		147 => sys_getsid(frame.ebx),
		158 => sys_sched_yield(),
		183 => sys_getcwd(frame.ebx, frame.ecx),
		192 => sys_mmap(
			frame.ebx,
			frame.ecx,
			frame.edx as i32,
			frame.esi as i32,
			frame.edi as i32,
			frame.ebp as isize,
		),
		199 => sys_getuid(),
		200 => sys_getgid(),
		212 => sys_chown(frame.ebx, frame.ecx, frame.edx),
		213 => sys_setuid(frame.ebx),
		214 => sys_setgid(frame.ebx),
		243 => sys_set_thread_area(frame.ebx),
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
		x => {
			trace_feature!(
				"syscall",
				"unimplemented syscall: {}(no={}) args:\n #1: {:#010x}\n #2: {:#010x}\n #3: {:#010x}\n #4: {:#010x}\n #5: {:#010x}\n #6: {:#010x}",
				get_syscall_name(x),
				x,
				frame.ebx,
				frame.ecx,
				frame.edx,
				frame.esi,
				frame.edi,
				frame.ebp
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
