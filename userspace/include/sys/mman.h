#ifndef _SYS_MMAN_H
#define _SYS_MMAN_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

#define MMAP_SHARED 0x01
#define MMAP_PRIVATE 0x02

#define PROT_READ 1
#define PROT_WRITE 2

DEFINE_SYSCALL(mmap, 90, void *, void *, addr, size_t, len, int, prot, int, flags, int, fd, off_t,
	       offset);
DEFINE_SYSCALL(munmap, 91, int, void *, addr, size_t, len);

#endif // _SYS_MMAN_H
