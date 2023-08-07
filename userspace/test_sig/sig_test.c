#include <kfs/kernel.h>
#include <kfs/ft.h>

void sig_int(int num) {
	(void)num;
	char *s = "hello\n";
	write(0, s, 6);
}

void sig_int_syscall(int sig) {
	(void)sig;

	char buf[3] = {
	    0,
	};
	// QUIT: hold this position.
	// INT : ignored(deferred) otherwise, same with SIGQUIT
	int ret = read(0, buf, 3);
	ft_putnbr(ret);
	write(0, "\nsig int syscall done\n", 22);
}

void sig_quit(int num) {
	(void)num;
	write(0, "sig quit!\n", 11);
}

void sig_action(int num, siginfo_t *info, void *sig_ctx) {
	(void)num;
	(void)sig_ctx;

	ft_putstr("sig action!\n");

	ft_putstr("==== sig info ====\n");

	ft_putstr("num: ");
	ft_putnbr_x(info->num);

	ft_putstr("  pid: "); // TODO '\t' not working.
	ft_putnbr_x(info->pid);

	ft_putstr("\nuid: ");
	ft_putnbr_x(info->uid);

	ft_putstr("  code: ");
	ft_putnbr_x(info->code);

	ucontext_t *u = sig_ctx;
	ft_putstr("\n==== sig context ====");

	ft_putstr("\nebp: ");
	ft_putnbr_x(u->ebp);
	ft_putstr("  edi: ");
	ft_putnbr_x(u->edi);
	ft_putstr("\nesi: ");
	ft_putnbr_x(u->esi);
	ft_putstr("  edx: ");
	ft_putnbr_x(u->edx);
	ft_putstr("\necx: ");
	ft_putnbr_x(u->ecx);
	ft_putstr("  ebx: ");
	ft_putnbr_x(u->ebx);
	ft_putstr("\neax: ");
	ft_putnbr_x(u->eax);
	ft_putstr("  ds: ");
	ft_putnbr_x(u->ds);
	ft_putstr("\nes: ");
	ft_putnbr_x(u->es);
	ft_putstr("  fs: ");
	ft_putnbr_x(u->fs);
	ft_putstr("\ngs: ");
	ft_putnbr_x(u->gs);
	ft_putstr("  handler: ");
	ft_putnbr_x(u->handler);
	ft_putstr("\nerror_code: ");
	ft_putnbr_x(u->error_code);
	ft_putstr("  eip: ");
	ft_putnbr_x(u->eip);
	ft_putstr("\ncs: ");
	ft_putnbr_x(u->cs);
	ft_putstr("  eflags: ");
	ft_putnbr_x(u->eflags);
	ft_putstr("\nesp: ");
	ft_putnbr_x(u->esp);
	ft_putstr("  ss: ");
	ft_putnbr_x(u->ss);
	ft_putstr("\nmask: ");
	ft_putnbr_x(u->mask);
	ft_putstr("  syscall_ret: ");
	ft_putnbr(u->syscall_ret);

	write(0, "\nsig action done\n", 17);
}

void next_test(int num) {
	char c = 0;

	write(0, "test", 4);
	ft_putnbr(num);

	while (c != '\n') {
		read(0, &c, 1);
	}

	write(0, "****done****\n", 13);
}

int main(void) {
	// 1
	// - check that signal handler is working.
	// - check returning to the interrupted position properly from signal handler after
	// processing signal.(next_test::read)
	signal(SIGINT, sig_int);
	signal(SIGQUIT, sig_quit);
	next_test(1);

	// 2
	// - check that DFL and IGN features is working.
	signal(SIGINT, SIG_DFL);  // sig_term! (pr_debug)
	signal(SIGQUIT, SIG_IGN); // nothing.
	next_test(2);

	// 3
	// - check that the mask feature is working.(SIGINT is ignored in handler)
	// - check returning to the interrupted position properly from signal handler after
	// processing signal.(sig_int_syscall::read)
	signal(SIGINT, sig_int_syscall);
	signal(SIGQUIT, sig_quit);
	next_test(3);

	// 4
	// - check the return value.
	if (signal(SIGINT, SIG_DFL) != sig_int_syscall)
		return 1;
	if (signal(SIGINT, SIG_IGN) != SIG_DFL)
		return 1;
	if (signal(SIGINT, sig_int) != SIG_IGN)
		return 1;
	next_test(4);

	// 5
	// - check actions when inputs is NULL
	struct sigaction a = {.sa_handler = sig_quit, .sa_mask = 0};
	struct sigaction b = {.sa_handler = 0, .sa_mask = 0};
	sigaction(SIGQUIT, &a, NULL);
	sigaction(SIGQUIT, NULL, &b);
	if (b.sa_handler == sig_quit) {
		write(0, "receive sig_quit\n", 17);
	}
	next_test(5);

	// 6
	// - check that the mask feature of sigaciton is working.(SIGINT, SIGQUIT)
	struct sigaction c = {.sa_handler = sig_int_syscall, .sa_mask = sigmask(SIGQUIT)};
	sigaction(SIGINT, &c, NULL);
	next_test(6);

	// 7
	// - check that the SA_NODEFER flag feature of sigaciton is working.(SIGINT)
	// - sinario: type SIGINT more than twice. After that, type enter.
	struct sigaction d = {
	    .sa_handler = sig_int_syscall, .sa_mask = sigmask(SIGQUIT), .sa_flags = SA_NODEFER};
	sigaction(SIGINT, &d, NULL);
	next_test(7);

	// 8
	// - check that the SA_RESTART flag feature of sigaciton is working.(SIGINT)
	// - sinario: type SIGINT more than twice. After that, type enter twice.
	struct sigaction e = {.sa_handler = sig_int_syscall,
			      .sa_mask = sigmask(SIGQUIT),
			      .sa_flags = SA_NODEFER | SA_RESTART};
	sigaction(SIGINT, &e, NULL);
	next_test(8);

	// 9
	// - check that the SA_RESETHAND flag feature of sigaciton is working.(SIGINT = SIGTERM)
	struct sigaction f = {
	    .sa_handler = sig_int_syscall, .sa_mask = 0, .sa_flags = SA_RESETHAND};
	sigaction(SIGINT, &f, NULL);
	next_test(9);

	// 10
	// - check that the SA_SIGINFO flag feature of sigaciton is working.(SIGINT = SIGTERM)
	// - check that the params of the signal handler is correct.
	struct sigaction g = {.sa_sigaction = sig_action, .sa_mask = 0, .sa_flags = SA_SIGINFO};
	sigaction(SIGINT, &g, NULL);
	next_test(10);

	return 0;
}
