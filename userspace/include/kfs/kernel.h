#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

typedef unsigned int size_t;
typedef int ssize_t;
typedef int pid_t;

void _exit(int code);
pid_t fork(void);
ssize_t read(int fildes, void *buf, size_t nbyte);
ssize_t wrire(int fildes, void *buf, size_t nbyte);
void fortytwo(int number);

#endif