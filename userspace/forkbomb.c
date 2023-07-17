#include <kfs/kernel.h>

int main(void) {
    for (;;) {
        pid_t cpid = fork();
        if (cpid != 0) {
            fortytwo(cpid);
            break;
        }
    }
    return 0;
}