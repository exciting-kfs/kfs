/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   ntoa.c                                             :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/24 01:18:05 by mypark            #+#    #+#             */
/*   Updated: 2022/01/04 02:19:24 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static size_t fit_type(int type, ssize_t x, int *minus) {
	if (x < 0 && is_in(type, "di")) {
		*minus = 1;
		x = -x;
	}
	if (type == 'p')
		return ((size_t)x);
	else
		return ((size_t)((unsigned int)x));
}

void ntoa_base(int type, ssize_t x, int base, t_stack *nbrst) {
	size_t num;
	char *digits;
	int minus;

	digits = "0123456789abcdef";
	nbrst->sp = 0;
	minus = 0;
	num = fit_type(type, x, &minus);
	nbrst->pool[nbrst->sp++] = digits[num % base];
	num /= base;
	while (num) {
		nbrst->pool[nbrst->sp++] = digits[num % base];
		num /= base;
	}
	if (minus)
		nbrst->pool[nbrst->sp++] = '-';
	nbrst->pool[nbrst->sp] = '\0';
}
