#ifndef _UNISTD_H
#define _UNISTD_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

DEFINE_SYSCALL(fork, 2, pid_t, void);
DEFINE_SYSCALL(_exit, 1, void, int, code);

DEFINE_SYSCALL(read, 3, ssize_t, int, fildes, void *, buf, size_t, nbyte);
DEFINE_SYSCALL(write, 4, size_t, int, fildes, const void *, buf, size_t, nbyte);
DEFINE_SYSCALL(close, 6, int, int, fildes);

DEFINE_SYSCALL(execve, 11, int, const char *, name, char *const *, argv, char *const *, envp);

DEFINE_SYSCALL(getpid, 20, pid_t, void);
DEFINE_SYSCALL(getppid, 64, pid_t, void);

DEFINE_SYSCALL(getpgrp, 65, pid_t, void);
DEFINE_SYSCALL(getpgid, 132, pid_t, pid_t, pid);
DEFINE_SYSCALL(setpgid, 57, pid_t, pid_t, pid, pid_t, pgid);

DEFINE_SYSCALL(setsid, 66, pid_t, void);
DEFINE_SYSCALL(getsid, 147, pid_t, pid_t, pid);

DEFINE_SYSCALL(pipe, 42, int, int *, pipe_pair);

DEFINE_SYSCALL(getuid, 199, uid_t, void);
DEFINE_SYSCALL(setuid, 213, int, uid_t, uid);

DEFINE_SYSCALL(getgid, 200, uid_t, void);
DEFINE_SYSCALL(setgid, 214, int, uid_t, gid);

DEFINE_SYSCALL(chdir, 12, int, const char *, path);
DEFINE_SYSCALL(getcwd, 183, char *, char *, buf, size_t, size);

DEFINE_SYSCALL(rmdir, 40, int, const char *, path);
DEFINE_SYSCALL(unlink, 10, int, const char *, path);
DEFINE_SYSCALL(symlink, 83, int, const char *, target, const char *, linkpath);

DEFINE_SYSCALL(reboot, 80, int, int, cmd);

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

DEFINE_SYSCALL(lseek, 19, off_t, int, fd, off_t, offset, int, whence);

DEFINE_SYSCALL(truncate, 92, int, const char *, path, off_t, length);

#endif // _UNISTD_H
