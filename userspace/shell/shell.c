#include <kfs/kernel.h>

int main(void) {
	char c;
	while (1) {
		read(0, &c, 1);
		write(0, &c, 1);
	}
	return 0;
}