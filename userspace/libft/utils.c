#include <unistd.h>

size_t ft_strlen(const char *s) {
	size_t i = 0;
	while (s[i]) {
		i++;
	}
	return i;
}

void ft_putstr(const char *s) {
	size_t n = ft_strlen(s);
	write(1, s, n);
}

#define __PUTNBR_BUFFER_MAX 32
#define __ABS(x) ((x) < 0 ? -(x) : (x))

int convert_to_str(char *buffer, int n, int base) {
	const char *digits = "0123456789abcdef";
	const int is_neg = n < 0;

	int curr = __PUTNBR_BUFFER_MAX - 1;
	buffer[curr--] = digits[__ABS(n % base)];
	n = __ABS(n / base);
	while (n) {
		buffer[curr--] = digits[(n % base)];
		n /= base;
	}

	if (is_neg)
		buffer[curr--] = '-';

	return curr + 1;
}

void ft_putnbr(int n) {
	char buffer[__PUTNBR_BUFFER_MAX];
	int start;

	start = convert_to_str(buffer, n, 10);
	write(1, buffer + start, __PUTNBR_BUFFER_MAX - start);
}

void ft_putnbr_x(int n) {
	char buffer[__PUTNBR_BUFFER_MAX];
	int start;

	start = convert_to_str(buffer, n, 16);
	write(1, buffer + start, __PUTNBR_BUFFER_MAX - start);
}
