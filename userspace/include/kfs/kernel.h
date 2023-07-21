#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

#define NULL ((void *)0)

typedef unsigned int size_t;
typedef int ssize_t;
typedef int pid_t;

void _exit(int code);
pid_t fork(void);

ssize_t read(int fildes, void *buf, size_t nbyte);
ssize_t write(int fildes, const void *buf, size_t nbyte);

int exec(const char *name);

pid_t waitpid(pid_t pid, int *stat_loc, int options);

#define _W_FLAG_MASK 0xff000000;
#define _W_STATUS_MASK 0x000000ff;

#define _W_SIGNALED 0x01000000;
#define _W_STOPPED 0x02000000;
#define _W_EXITED 0x03000000;
#define _W_CORE_DUMPED 0x04000000;

#define _W_GET_FLAG(x) ((x) & _W_FLAG_MASK)
#define _W_GET_STATUS(x) ((x) & _W_STATUS_MASK)

#define WIFEXITED(x) (_W_GET_FLAG(x) == _W_EXITED)
#define WIFSIGNALED(x) (_W_GET_FLAG(x) == _W_SIGNALED)
#define WIFSTOPPED(x) (_W_GET_FLAG(x) == _W_STOPPED)
#define WCOREDUMP(x) (_W_GET_FLAG(x) == _W_CORE_DUMPED)

#define WEXITSTATUS(x) _W_GET_STATUS(x)
#define WTERMSIG(x) _W_GET_STATUS(x)
#define WSTOPSIG(x) _W_GET_STATUS(x)

#define WNOHANG (1<<0);
#define WUNTRACED (1<<1);

pid_t getpid(void);
pid_t getppid(void);

pid_t getpgrp(void);
pid_t getpgid(pid_t pid);
pid_t setpgid(pid_t pid, pid_t pgid);

pid_t setsid(void);
pid_t getsid(void);

void fortytwo(int number);

#define SIGINT 2
#define SIGQUIT 3
#define SIG_DFL (void *)0
#define SIG_IGN (void *)1

#define sigmask(m) (1 << ((m)-1))

#define SA_ONSTACK 0x0001   /* take signal on signal stack */
#define SA_RESTART 0x0002   /* restart system on signal return */
#define SA_RESETHAND 0x0004 /* reset to SIG_DFL when taking signal */
#define SA_NOCLDSTOP 0x0008 /* do not generate SIGCHLD on child stop */
#define SA_NODEFER 0x0010   /* don't mask the signal we're delivering */
#define SA_NOCLDWAIT 0x0020 /* don't keep zombies around */
#define SA_SIGINFO 0x0040   /* signal handler with SA_SIGINFO args */

typedef void (*sighandler_t)(int);
typedef size_t sigset_t;

typedef struct siginfo {
	size_t num;
	size_t pid;
	size_t uid;
	size_t code;
} siginfo_t;

typedef struct ucontext {
	size_t ebp;
	size_t edi;
	size_t esi;
	size_t edx;
	size_t ecx;
	size_t ebx;
	size_t eax;
	size_t ds;
	size_t es;
	size_t fs;
	size_t gs;
	size_t handler;
	size_t error_code;
	size_t eip;
	size_t cs;
	size_t eflags;
	size_t esp;
	size_t ss;
	sigset_t mask;
	ssize_t syscall_ret;
} ucontext_t;

struct sigaction {
	void (*sa_handler)(int);
	void (*sa_sigaction)(int, siginfo_t *, void *);
	sigset_t sa_mask;
	int sa_flags;
};

sighandler_t signal(int signum, sighandler_t handler);
int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);

#endif
