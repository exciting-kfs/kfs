/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   ft_printf.c                                        :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/21 02:25:01 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 18:54:36 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

t_state *printer(int *sum, const char c, t_optable *otb, va_list *ap) {
	static t_state s = c_write;

	if (s == c_write)
		s = char_writer(sum, c);
	else if (s == o_read)
		s = opts_reader(sum, c, otb, ap);
	return (&s);
}

int ft_printf(const char *fmt, ...) {
	va_list ap;
	t_state *s;
	t_optable otb;
	int sum;

	sum = 0;
	opts_initialization(&otb);
	fill_action(&otb);

	va_start(ap, fmt);
	while (*fmt) {
		s = printer(&sum, *fmt, &otb, &ap);
		fmt++;
	}
	*s = c_write;
	va_end(ap);
	return (sum);
}
