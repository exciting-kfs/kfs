#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

#include <time.h>


struct stat {
	mode_t perm;
	uid_t uid;
	gid_t gid;
	off_t size;
	unsigned int file_type;
	struct timespec access_time;
	struct timespec modify_time;
	struct timespec change_time;
};

DEFINE_SYSCALL(mkdir, 39, int, const char *, path, mode_t, mode);
DEFINE_SYSCALL(stat, 18, int, const char *, path, struct stat *, statbuf);
DEFINE_SYSCALL(chmod, 15, int, const char *, path, mode_t, mode);
DEFINE_SYSCALL(chown, 212, int, const char *, path, uid_t, owner, gid_t, group);

#endif // _SYS_STAT_H
