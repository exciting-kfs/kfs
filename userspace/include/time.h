#ifndef _TIME_H
#define _TIME_H

#include "kfs/internal/prelude.h"

struct timespec {
    time_t tv_sec;
    int tv_nsec;
};

#endif // _TIME_H
