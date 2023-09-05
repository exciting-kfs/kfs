#ifndef _SYS_STAT_H
#define _SYS_STAT_H

#include "kfs/internal/prelude.h"

int mkdir(const char *path, mode_t mode);

struct stat {
	unsigned int perm;
	uid_t uid;
	gid_t gid;
	off_t size;
};

int stat(const char *path, struct stat *statbuf);
int chmod(const char *path, mode_t mode);
int chown(const char *path, uid_t owner, gid_t group);

#endif // _SYS_STAT_H
