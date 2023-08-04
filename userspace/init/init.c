#include <kfs/kernel.h>

// we don't have .BSS yet :(

int main(void) {

	int pipe_pair[2];
	int ret = pipe(pipe_pair);
	if (ret)
		return 1;

	int pid = fork();

	if (pid == 0) {
		exec("test_sig_stop_cont.bin");
	}

	for (;;) {
		sched_yield();
	}

	return 0;
}
