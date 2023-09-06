#ifndef _FCNTL_H
#define _FCNTL_H

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

int open(const char *path, int flags, ...);
int creat(const char *path, int mode);

#endif // _FCNTL_H
