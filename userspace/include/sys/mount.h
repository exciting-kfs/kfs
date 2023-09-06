#ifndef _SYS_MOUNT_H
#define _SYS_MOUNT_H

#include "kfs/internal/prelude.h"

int mount(const char *path, const char *fs_name);
int umount(const char *path);

#endif // _SYS_MOUNT_H
