#include <kfs/kernel.h>

void sig_int(int num) {
	(void)num;
	char *s = "hello\n";
	write(0, s, 6);
}

void sig_quit(int num) {
	(void)num;
	write(0, "sig quit!\n", 11);
}

int main(void) {
	// // 1
	// signal(SIGINT, sig_int);
	// signal(SIGQUIT, sig_quit);

	// // 2
	// signal(SIGINT, SIG_DFL);
	// signal(SIGQUIT, SIG_IGN);

	// // 3
	// struct sigaction a = {.sa_handler = sig_quit, .sa_mask = 0};
	// struct sigaction b = {.sa_handler = 0, .sa_mask = 0};
	// sigaction(SIGQUIT, &a, NULL);
	// sigaction(SIGQUIT, NULL, &b);
	// if (b.sa_handler == sig_quit) {
	// 	write(0, "receive sig_quit\n", 18);
	// }

	char c;
	while (1) {
		read(0, &c, 1);
		write(0, &c, 1);
	}
	return 0;
}
