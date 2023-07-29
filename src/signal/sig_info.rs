use super::{sig_code::SigCode, sig_num::SigNum};

#[derive(Debug, Clone)]
#[repr(C)]
pub struct SigInfo {
	pub num: SigNum,   /* Signal number */
	pub pid: usize,    /* Sending process ID */
	pub uid: usize,    /* Real user ID of sending process */
	pub code: SigCode, /* Signal code: why this signal was sent. */
}

// struct sig_info {
// 	...
// int      errno;        /* An errno value */
// int      trapno        /* Trap number that caused hardware-generated signal (unused on most architectures) */
// int      si_status;    /* Exit value or signal */
// clock_t  si_utime;     /* User time consumed */
// clock_t  si_stime;     /* System time consumed */
// union sigval si_value; /* Signal value */
// int      si_int;       /* POSIX.1b signal */
// void    *si_ptr;       /* POSIX.1b signal */
// int      si_overrun;   /* Timer overrun count;  POSIX.1b timers */
// int      si_timerid;   /* Timer ID; POSIX.1b timers */
// void    *si_addr;      /* Memory location which caused fault */
// long     si_band;      /* Band event (was int in glibc 2.3.2 and earlier) */
// int      si_fd;        /* File descriptor */
// short    si_addr_lsb;  /* Least significant bit of address (since Linux 2.6.32) */
// void    *si_lower;     /* Lower bound when address violation occurred (since Linux 3.19) */
// void    *si_upper;     /* Upper bound when address violation occurred (since Linux 3.19) */
// int      si_pkey;      /* Protection key on PTE that caused fault (since Linux 4.6) */
// void    *si_call_addr; /* Address of system call instruction (since Linux 3.5) */
// int      si_syscall;   /* Number of attempted system call (since Linux 3.5) */
// unsigned int si_arch;  /* Architecture of attempted system call
// }
