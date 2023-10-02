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

uid_t getgid(void);
int setgid(uid_t gid);

int chdir(const char *path);
char *getcwd(char *buf, size_t size);

int rmdir(const char *path);
int unlink(const char *path);
int symlink(const char *target, const char *linkpath);

#define SEEK_SET 0
#define SEEK_CUR 1
#define SEEK_END 2

off_t lseek(int fd, off_t offset, int whence);

int truncate(const char *path, off_t length);

#endif // _UNISTD_H
