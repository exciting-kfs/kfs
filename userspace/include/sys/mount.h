#ifndef _SYS_MOUNT_H
#define _SYS_MOUNT_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

DEFINE_SYSCALL(mount, 21, int, const char *, path, const char *, fs_name);
DEFINE_SYSCALL(umount, 22, int, const char *, path);

#endif // _SYS_MOUNT_H
