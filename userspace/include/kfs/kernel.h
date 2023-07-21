#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

#define NULL ((void *)0)

typedef unsigned int size_t;
typedef int ssize_t;
typedef int pid_t;

void _exit(int code);
pid_t fork(void);
ssize_t read(int fildes, void *buf, size_t nbyte);
ssize_t write(int fildes, void *buf, size_t nbyte);
pid_t waitpid(pid_t pid, int *stat_loc, int options);
void fortytwo(int number);

#endif
