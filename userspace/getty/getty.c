#include "kfs/ft.h"
#include "kfs/libft.h"

char *__crypt_sha512(const char *key, const char *setting, char *output);

int main(void) {
	char outbuf[128];

	// ft_putstr("HASH = ");
	ft_putstr(__crypt_sha512("root", "$6$xKfGiVIDU2eFHpz9$", outbuf));
	// ft_printf("HASH = %s\n", );
	while(2);
}