#include "kfs/libft.h"
#include <signal.h>
#include <time.h>

void sigint_handler(int sig) {
	(void)sig;
	ft_printf("SIG INT\n");
}

int main() {

	signal(SIGINT, sigint_handler);

	struct timespec start, end, remain;

	clock_gettime(CLOCK_REALTIME, &start);

	ft_printf("start: %d second\n", start.tv_sec);

	start.tv_sec += 4;
	int ret = nanosleep(&start, &remain);

	if (ret < 0) {
		ft_printf("nanosleep faild\n");
		ft_printf("remain: %d\n", remain.tv_sec);
	}

	clock_gettime(CLOCK_REALTIME, &end);

	ft_printf("end: %d second\n", end.tv_sec);
}