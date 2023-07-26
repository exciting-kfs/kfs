#include <kfs/kernel.h>

void sig_int(int num) {
	(void)num;
	char *s = "hello\n";
	write(0, s, 6);
}

void sig_int2(int sig) {
	(void)sig;

	char buf[3] = {
	    0,
	};
	// INT : ignore. (DEFER)
	// QUIT: hold this position.
	read(0, buf, 3);
	write(0, "sig int2 done\n", 15);
}

void sig_quit(int num) {
	(void)num;
	write(0, "sig quit!\n", 11);
}

void next_test(int num) {
	num += '0';
	char c = 0;
	while (c != '\n') {
		read(0, &c, 1);
	}
	write(0, "test", 4);
	write(0, &num, 1);
	write(0, " done\n", 6);
}

int main(void) {
	// 1
	// - check that signal handler is working.
	// - check returning to interrupted position properly from signal handler after processing
	// signal.(next_test::read)
	signal(SIGINT, sig_int);
	signal(SIGQUIT, sig_quit);
	next_test(1);

	// 2
	// - check that DFL and IGN features is working.
	signal(SIGINT, SIG_DFL);  // sig_term!
	signal(SIGQUIT, SIG_IGN); // nothing.
	next_test(2);

	// 3
	// - check that the mask feature is working.(SIGINT)
	// - check returning to interrupted position properly from signal handler after processing
	// signal.(sig_int2::read)
	signal(SIGINT, sig_int2);
	next_test(3);

	// 4
	// - check the return value.
	if (signal(SIGINT, SIG_DFL) != sig_int2)
		return 1;
	if (signal(SIGINT, SIG_IGN) != SIG_DFL)
		return 1;
	if (signal(SIGINT, sig_int) != SIG_IGN)
		return 1;
	next_test(4);

	// // 3
	// struct sigaction a = {.sa_handler = sig_quit, .sa_mask = 0};
	// struct sigaction b = {.sa_handler = 0, .sa_mask = 0};
	// sigaction(SIGQUIT, &a, NULL);
	// sigaction(SIGQUIT, NULL, &b);
	// if (b.sa_handler == sig_quit) {
	// 	write(0, "receive sig_quit\n", 18);
	// }

	// 4
	// struct sigaction a = {.sa_handler = sig_quit, .sa_mask = 0};
	// struct sigaction b = {.sa_handler = sig_quit, .sa_mask = 0, .sa_flags = SA_RESETHAND};
	// sigaction(SIGQUIT, &a, NULL);
	// sigaction(SIGQUIT, &b, NULL);

	write(0, "loop\n", 5);
	while (1) {
	}
	return 0;
}
