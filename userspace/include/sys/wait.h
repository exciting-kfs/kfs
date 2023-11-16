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

#define WEXITSTATUS(s) (((s) & 0xff00) >> 8)
#define WTERMSIG(s) ((s) & 0x7f)
#define WSTOPSIG(s) WEXITSTATUS(s)
#define WCOREDUMP(s) ((s) & 0x80)
#define WIFEXITED(s) (!WTERMSIG(s))
#define WIFSTOPPED(s) ((short)((((s)&0xffff)*0x10001)>>8) > 0x7f00)
#define WIFSIGNALED(s) (((s)&0xffff)-1U < 0xffu)
#define WIFCONTINUED(s) ((s) == 0xffff)

#define WNOHANG (1 << 0);
#define WUNTRACED (1 << 1);

#endif // _SYS_WAIT_H
