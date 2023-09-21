/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   my_putnbr.c                                        :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/28 22:36:02 by mypark            #+#    #+#             */
/*   Updated: 2022/01/04 15:57:05 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static void	push_prefix(char type, int minus, \
						t_stack *nbrst, const t_optable *otb)
{
	char	top;

	top = nbrst->pool[nbrst->sp - 1];
	if (is_in(type, "udixX") && top == '0' && otb->preci == 0)
		nbrst->sp--;
	if (is_in(type, "xX") && top == '0')
		return ;
	if (minus)
		nbrst->pool[nbrst->sp++] = '-';
	else if (otb->base)
	{
		nbrst->pool[nbrst->sp++] = 'x';
		nbrst->pool[nbrst->sp++] = '0';
	}
	else if (otb->plus)
		nbrst->pool[nbrst->sp++] = '+';
	else if (otb->space)
		nbrst->pool[nbrst->sp++] = ' ';
	nbrst->pool[nbrst->sp] = '\0';
}

static void	push_nbr(char type, ssize_t num, int *minus, t_stack *nbrst)
{
	if (is_in(type, "udi"))
		ntoa_base(type, num, 10, nbrst);
	else
		ntoa_base(type, num, 16, nbrst);
	*minus = 0;
	if (nbrst->pool[nbrst->sp - 1] == '-')
	{
		*minus = 1;
		nbrst->sp--;
		nbrst->pool[nbrst->sp] = '\0';
	}
}

int	my_putnbr(char type, const t_optable *otb, va_list ap)
{
	t_stack	nbrst;
	ssize_t	num;
	int		len;
	int		minus;

	if (type == 'p')
		num = va_arg(ap, size_t);
	else
		num = va_arg(ap, int);
	push_nbr(type, num, &minus, &nbrst);
	nbrst.bp = nbrst.sp - 1;
	push_prefix(type, minus, &nbrst, otb);
	if (type == 'X')
		ft_strtoupper(nbrst.pool);
	len = print_stack(&nbrst, otb);
	return (len);
}
