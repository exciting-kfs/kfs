#ifndef _FCNTL_H
#define _FCNTL_H

#include <stdarg.h>

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

#define O_RDONLY (0)
#define O_WRONLY (1)
#define O_RDWR (2)
#define O_CREAT (0100)
#define O_EXCL (0200)
#define O_NOCTTY (0400)
#define O_TRUNC (01000)
#define O_DIRECTORY (0200000)
#define O_NOFOLLOW (0400000)
#define O_CLOEXEC (02000000)
#define O_APPEND (02000)
#define O_NONBLOCK (04000)
#define O_SYNC (010000)

#define AT_FDCWD (-100)
#define AT_EMPTY_PATH (0x1000)
#define AT_SYMLINK_NOFOLLOW (0x100)

#define S_IFMT (0170000)

#define S_IFSOCK (0140000)
#define S_IFLNK (0120000)
#define S_IFREG (0100000)
#define S_IFBLK (0060000)
#define S_IFDIR (0040000)
#define S_IFCHR (0020000)
#define S_IFIFO (0010000)

static inline int open(const char *path, int flags, ...) {
	mode_t mode = 0;

	if (flags & O_CREAT) {
		va_list ap;
		va_start(ap, flags);
		mode |= va_arg(ap, mode_t);
		va_end(ap);
	}

	return (int)__syscall3(5, (long)path, (long)flags, (long)mode);
}

DEFINE_SYSCALL(creat, 8, int, const char *, path, int, mode);

#endif // _FCNTL_H
