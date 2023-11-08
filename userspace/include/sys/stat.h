#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

#include <time.h>

struct statx {
	unsigned int mask;
	unsigned int blksize;
	unsigned long long attributes;
	unsigned int nlink;
	unsigned int uid;
	unsigned int gid;
	unsigned short mode;
	unsigned short pad1;
	unsigned long long ino;
	unsigned long long size;
	unsigned long long blocks;
	unsigned long long attributes_mask;
	struct {
		long long sec;
		unsigned int nsec;
		int pad;
	} atime, btime, ctime, mtime;
	unsigned int rdev_major;
	unsigned int rdev_minor;
	unsigned int dev_major;
	unsigned int dev_minor;
};

#define STATX_ALL 0xfff

DEFINE_SYSCALL(mkdir, 39, int, const char *, path, mode_t, mode);
DEFINE_SYSCALL(statx, 383, int, int, dirfd, const char *, path, int, flags, int, mask, struct statx *, stat_buf);
DEFINE_SYSCALL(chmod, 15, int, const char *, path, mode_t, mode);
DEFINE_SYSCALL(chown, 212, int, const char *, path, uid_t, owner, gid_t, group);

#endif // _SYS_STAT_H
