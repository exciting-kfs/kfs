#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

#include "kfs/internal/prelude.h"

int sched_yield(void);

struct kfs_dirent {
    unsigned int ino;
    unsigned int private;
    unsigned short size;
    char name[0];
};

ssize_t getdents(int fd, void *dirp, size_t len);

#endif // _KFS_KERNEL_H
