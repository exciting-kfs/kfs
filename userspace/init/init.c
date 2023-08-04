#include <kfs/kernel.h>

int main(void) {

	int pid = fork();
	if (pid == 0) {
		void *p = mmap(NULL, 4096, PROT_READ | PROT_WRITE, MMAP_PRIVATE, -1, 0);

		// *(unsigned char *)p = 42;
		*(unsigned char *)p = 42;
		exec("test_sig_stop_cont.bin");
		_exit(1);
	}

	for (;;) {
		sched_yield();
	}

	return 0;
}
