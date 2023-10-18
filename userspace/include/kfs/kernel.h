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

#define	KFS_POWER_OFF 0x4321fedc
#define	KFS_RESTART 0x1234567

DEFINE_SYSCALL(init_module, 128, int, const char *, path);
DEFINE_SYSCALL(cleanup_module, 129, int, const char *, name);

#endif // _KFS_KERNEL_H
