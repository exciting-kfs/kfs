#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#include "kfs/internal/prelude.h"

#include <time.h>

int mkdir(const char *path, mode_t mode);

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

int stat(const char *path, struct stat *statbuf);
int chmod(const char *path, mode_t mode);
int chown(const char *path, uid_t owner, gid_t group);

#endif // _SYS_STAT_H
