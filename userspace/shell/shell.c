#include <kfs/kernel.h>

int main(void) {

	write(0, "loop\n", 5);
	while (1) {
	}
	return 0;
}
