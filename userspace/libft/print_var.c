/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   print_var.c                                        :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/28 21:41:35 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 20:05:22 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static int my_putchar(const t_optable *otb, va_list *ap) {
	char c;
	int len;

	c = va_arg(*ap, int);
	len = otb->width - 1;
	if (otb->align == right)
		len = print_ws(len, ' ');
	write(1, &c, 1);
	if (otb->align == left)
		len = print_ws(len, ' ');
	return (len + 1);
}

static int my_putstr(const t_optable *otb, va_list *ap) {
	int len_tot;
	int len_s;
	size_t n;
	char *s;

	len_tot = 0;
	s = va_arg(*ap, char *);
	if (s != NULL)
		len_s = ft_strlen(s);
	else {
		s = "(null)";
		len_s = 6;
	}
	if (otb->align == right)
		len_tot += print_ws(otb->width - len_s, ' ');
	n = 0;
	while (s[n] && n < otb->preci)
		write(1, s + (n++), 1);
	len_tot += n;
	if (otb->align == left)
		len_tot += print_ws(otb->width - len_s, ' ');
	return (len_tot);
}

static int my_putelse(const t_optable *otb, char type) {
	int len;
	char w;

	w = ' ';
	if (otb->zero)
		w = '0';
	len = otb->width - 1;
	if (otb->align == right)
		len = print_ws(len, w);
	write(1, &type, 1);
	if (otb->align == left)
		len = print_ws(len, w);
	return (len + 1);
}

int print_var(char type, const t_optable *otb, va_list *ap) {
	int len;

	len = 0;
	if (type == 'c')
		len = my_putchar(otb, ap);
	else if (type == 's')
		len = my_putstr(otb, ap);
	else if (is_in(type, "udixXp"))
		len = my_putnbr(type, otb, ap);
	else
		len = my_putelse(otb, type);
	return (len);
}
