/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   print_stack.c                                      :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/28 22:36:02 by mypark            #+#    #+#             */
/*   Updated: 2022/01/04 02:19:24 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

static void	set_width_spec(char *ws, int *ws_p, \
							t_stack *nbrst, const t_optable *otb)
{
	int	top;
	int	mid;

	top = nbrst->sp - 1;
	mid = nbrst->bp;
	*ws = ' ';
	if (otb->align == right && otb->zero)
	{
		*ws = '0';
		*ws_p = mid;
	}
	else if (otb->align == right)
		*ws_p = top;
}

static void	set_preci_spec(int *len_preci, \
							t_stack *nbrst, const t_optable *otb)
{
	int	len_nbr;

	len_nbr = nbrst->bp + 1;
	if ((int)otb->preci - len_nbr > 0)
		*len_preci = otb->preci - len_nbr;
	else
		*len_preci = 0;
}

int	print_stack(t_stack *nbrst, const t_optable *otb)
{
	int		len;
	int		len_preci;
	int		ws_p;
	char	ws;

	len = nbrst->sp;
	set_preci_spec(&len_preci, nbrst, otb);
	set_width_spec(&ws, &ws_p, nbrst, otb);
	while (nbrst->sp--)
	{
		if (nbrst->sp == ws_p)
			len += print_ws(otb->width - len - len_preci, ws);
		if (nbrst->sp == nbrst->bp)
			len += print_ws(len_preci, '0');
		write(1, nbrst->pool + nbrst->sp, 1);
	}
	if (otb->align == left)
		len += print_ws(otb->width - len, ' ');
	return (len);
}
