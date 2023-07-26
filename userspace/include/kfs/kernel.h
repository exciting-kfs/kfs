#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

#define NULL (void *)0

typedef unsigned int size_t;
typedef int ssize_t;
typedef int pid_t;

void _exit(int code);
pid_t fork(void);

ssize_t read(int fildes, void *buf, size_t nbyte);
ssize_t write(int fildes, void *buf, size_t nbyte);

int exec(const char *name);

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

struct sigaction {
	void (*sa_handler)(int);
	void (*sa_sigaction)(int, siginfo_t *, void *);
	sigset_t sa_mask;
	int sa_flags;
};

sighandler_t signal(int signum, sighandler_t handler);
int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);

#endif
