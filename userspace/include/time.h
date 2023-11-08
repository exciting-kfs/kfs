#ifndef _TIME_H
#define _TIME_H

#include "kfs/internal/prelude.h"
#include "kfs/syscall.h"

struct timespec {
	time_t tv_sec;
	int tv_nsec;
};

typedef int clockid_t;

DEFINE_SYSCALL(clock_gettime, 265, int, clockid_t, clk_id, struct timespec *, tp);
DEFINE_SYSCALL(nanosleep, 162, int, const struct timespec *, req, struct timespec *, rem);

#define CLOCK_REALTIME 0
#define CLOCK_MONOTONIC 1

#endif // _TIME_H
