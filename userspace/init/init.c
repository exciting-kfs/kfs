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
	for (;;)
		;
}

int main(void) {
	for (int i = 0; i < 5; i++) {
		for (int j = 0; j < 5; j++) {
			pid_t *child = &childs[i][j];
			*child = fork();

			if (*child == 0) {
				// no_return
				do_something();
			}

			fortytwo(*child);
			setpgid(*child, childs[i][0]);
		}
	}

	fortytwo(1111111111);
	fortytwo(1111111111);

	kill_all_by_pid();
	// kill_all_by_pgroup();
	// kill_all_by_wildcard();

	pid_t child;
	int status;
	for (;;) {
		while ((child = waitpid(-1, &status, 0)) > 0) {
			fortytwo(WTERMSIG(status));
			fortytwo(child);
		}
	}
	return 0;
}
