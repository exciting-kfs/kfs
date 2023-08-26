#ifndef _SYS_MMAN_H
#define _SYS_MMAN_H

#include "kfs/internal/prelude.h"

#define MMAP_PRIVATE 0x02

#define PROT_READ 1
#define PROT_WRITE 2

void *mmap(void *addr, size_t len, int prot, int flags, int fd, off_t offset);

#endif // _SYS_MMAN_H
