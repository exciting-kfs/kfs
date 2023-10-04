#ifndef _KFS_KERNEL_H
#define _KFS_KERNEL_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

DEFINE_SYSCALL(sched_yield, 158, int, void);

struct kfs_dirent {
	unsigned int ino;
	unsigned int private;
	unsigned short size;
	unsigned char file_type;
	char name[0];
};

DEFINE_SYSCALL(getdents, 141, ssize_t, int, fd, void *, dirp, size_t, len);

#endif // _KFS_KERNEL_H
