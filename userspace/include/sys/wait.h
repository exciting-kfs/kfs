#ifndef _SYS_WAIT_H
#define _SYS_WAIT_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

DEFINE_SYSCALL(waitpid, 7, pid_t, pid_t, pid, int *, stat_loc, int, options);

#define _W_FLAG_MASK 0xff000000
#define _W_STATUS_MASK 0x000000ff

#define _W_SIGNALED 0x01000000
#define _W_STOPPED 0x02000000
#define _W_EXITED 0x03000000
#define _W_CORE_DUMPED 0x04000000

#define _W_GET_FLAG(x) ((x)&_W_FLAG_MASK)
#define _W_GET_STATUS(x) ((x)&_W_STATUS_MASK)

#define WIFEXITED(x) (_W_GET_FLAG(x) == _W_EXITED)
#define WIFSIGNALED(x) (_W_GET_FLAG(x) == _W_SIGNALED)
#define WIFSTOPPED(x) (_W_GET_FLAG(x) == _W_STOPPED)
#define WCOREDUMP(x) (_W_GET_FLAG(x) == _W_CORE_DUMPED)

#define WEXITSTATUS(x) _W_GET_STATUS(x)
#define WTERMSIG(x) _W_GET_STATUS(x)
#define WSTOPSIG(x) _W_GET_STATUS(x)

#define WNOHANG (1 << 0);
#define WUNTRACED (1 << 1);

#endif // _SYS_WAIT_H
