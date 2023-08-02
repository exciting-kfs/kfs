#include <kfs/kernel.h>

int main(void) {

	write(0, "loop\n", 5);
	char c;
	while (1) {
		read(0, &c, 1);
		write(0, &c, 1);
	}
	return 0;
}
