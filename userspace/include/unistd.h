#ifndef _UNISTD_H
#define _UNISTD_H

#include "kfs/internal/prelude.h"

pid_t fork(void);
void _exit(int code);

ssize_t read(int fildes, void *buf, size_t nbyte);
ssize_t write(int fildes, const void *buf, size_t nbyte);
int close(int fildes);

int exec(const char *name);

pid_t getpid(void);
pid_t getppid(void);

pid_t getpgrp(void);
pid_t getpgid(pid_t pid);
pid_t setpgid(pid_t pid, pid_t pgid);

pid_t setsid(void);
pid_t getsid(void);

int pipe(int pipe_pair[2]);

uid_t getuid(void);
int setuid(uid_t uid);

#endif // _UNISTD_H
