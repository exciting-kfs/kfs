#include <kfs/kernel.h>

void show_relation(void) {
	pid_t pid = getpid();
	pid_t ppid = getppid();
	pid_t pgid = getpgid(pid);
	pid_t sid = getsid();

	fortytwo(pid);
	fortytwo(ppid);
	fortytwo(pgid);
	fortytwo(sid);
}

int do_child(void) {
	show_relation();

	return 0;
}

int main(void) {
	pid_t childs[5];
	for (int i = 0; i < 5; i++) {
		childs[i] = fork();
		if (childs[i] == 0) {
			setpgid(0, 0);
			_exit(do_child());
		}
	}

	int status;
	for (int i = 0; i < 5; i++) {
		pid_t pid = waitpid(childs[i], &status, 0);
		fortytwo(pid);
		fortytwo(status);
	}
	return 0;
}