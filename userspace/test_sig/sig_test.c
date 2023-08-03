#include <kfs/kernel.h>

const char *base_10 = "0123456789";
const char *base_16 = "0123456789abcdef";

void ft_putnbr_x(size_t num) {
	if (num / 16) {
		ft_putnbr_x(num / 16);
	}
	write(0, &base_16[num % 16], 1);
}

void ft_putnbr(int num) {

	if (num < 0) {
		write(0, "-", 1);
		num = -num;
	}

	if (num / 10) {
		ft_putnbr(num / 10);
	}
	write(0, &base_10[num % 10], 1);
}

size_t ft_strlen(const char *s) {
	size_t i = 0;
	while (s[i]) {
		i++;
	}
	return i;
}

void ft_putstr_fd(int fd, const char *s) {
	size_t n = ft_strlen(s);
	write(fd, s, n);
}

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

	write(0, "sig action!\n", 13);

	ft_putstr_fd(0, "==== sig info ====\n");

	ft_putstr_fd(0, "num: ");
	ft_putnbr_x(info->num);

	ft_putstr_fd(0, "  pid: "); // TODO '\t' not working.
	ft_putnbr_x(info->pid);

	ft_putstr_fd(0, "\nuid: ");
	ft_putnbr_x(info->uid);

	ft_putstr_fd(0, "  code: ");
	ft_putnbr_x(info->code);

	ucontext_t *u = sig_ctx;
	ft_putstr_fd(0, "\n==== sig context ====");

	ft_putstr_fd(0, "\nebp: ");
	ft_putnbr_x(u->ebp);
	ft_putstr_fd(0, "  edi: ");
	ft_putnbr_x(u->edi);
	ft_putstr_fd(0, "\nesi: ");
	ft_putnbr_x(u->esi);
	ft_putstr_fd(0, "  edx: ");
	ft_putnbr_x(u->edx);
	ft_putstr_fd(0, "\necx: ");
	ft_putnbr_x(u->ecx);
	ft_putstr_fd(0, "  ebx: ");
	ft_putnbr_x(u->ebx);
	ft_putstr_fd(0, "\neax: ");
	ft_putnbr_x(u->eax);
	ft_putstr_fd(0, "  ds: ");
	ft_putnbr_x(u->ds);
	ft_putstr_fd(0, "\nes: ");
	ft_putnbr_x(u->es);
	ft_putstr_fd(0, "  fs: ");
	ft_putnbr_x(u->fs);
	ft_putstr_fd(0, "\ngs: ");
	ft_putnbr_x(u->gs);
	ft_putstr_fd(0, "  handler: ");
	ft_putnbr_x(u->handler);
	ft_putstr_fd(0, "\nerror_code: ");
	ft_putnbr_x(u->error_code);
	ft_putstr_fd(0, "  eip: ");
	ft_putnbr_x(u->eip);
	ft_putstr_fd(0, "\ncs: ");
	ft_putnbr_x(u->cs);
	ft_putstr_fd(0, "  eflags: ");
	ft_putnbr_x(u->eflags);
	ft_putstr_fd(0, "\nesp: ");
	ft_putnbr_x(u->esp);
	ft_putstr_fd(0, "  ss: ");
	ft_putnbr_x(u->ss);
	ft_putstr_fd(0, "\nmask: ");
	ft_putnbr_x(u->mask);
	ft_putstr_fd(0, "  syscall_ret: ");
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

	write(0, "loop\n", 5);
	while (1) {
	}
	return 0;
}
