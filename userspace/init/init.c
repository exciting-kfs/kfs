#include <kfs/kernel.h>

// we don't have .BSS yet :(
pid_t childs[5][5] = {
    {1}, {1}, {1}, {1}, {1},
};

void kill_all_by_pid(void) {
	for (int i = 0; i < 5; i++) {
		for (int j = 0; j < 5; j++) {
			kill(childs[i][j], SIGTERM);
		}
	}
}

void kill_all_by_pgroup(void) {
	for (int i = 0; i < 5; i++) {
		kill(-childs[i][0], SIGTERM);
	}
}

void kill_all_by_wildcard(void) {
	kill(-1, SIGTERM);
}

void do_something(void) {
	fortytwo(-getuid());
	for (;;)
		;
}

int main(void) {
	int pid = fork();
	if (pid == 0) {
		exec("shell.bin");
	}

	while (1) {
		sched_yield();
	}
	return 0;
}
