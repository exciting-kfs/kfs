/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   opts_handle.c                                      :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/24 00:55:24 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 20:06:31 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

void	opts_correction(int type, t_optable *otb)
{
	if (type == 's' && otb->p_chk == 0)
		otb->preci = (size_t)(-1);
	if (!(type == 'x' || type == 'X'))
		otb->base = 0;
	if (type == 'p')
		otb->base = 1;
	if (!is_in(type, "dip"))
	{
		otb->space = 0;
		otb->plus = 0;
	}
}

void	opts_initialization(t_optable *otb)
{
	otb->space = 0;
	otb->plus = 0;
	otb->base = 0;
	otb->zero = 0;
	otb->align = right;
	otb->width = 0;
	otb->preci = 1;
	otb->p_chk = 0;
	otb->init = 1;
}

void	fill_action(t_optable *otb)
{
	char	c;
	int		i;
	int		j;

	c = '0';
	while (c <= '9')
	{
		otb->action[flags][(int)c] = goto_width;
		otb->action[width][(int)c] = append_width;
		otb->action[preci][(int)c] = append_preci;
		c++;
	}
	otb->action[flags][(int) '0'] = fill_optable;
	otb->action[flags][(int) '.'] = goto_preci;
	otb->action[width][(int) '.'] = goto_preci;
	otb->action[preci][(int) '.'] = goto_preci;
	i = 0;
	while (i < 3)
	{
		j = 0;
		while (j < 4)
			otb->action[i][(int)("-+ #"[j++])] = fill_optable;
		i++;
	}
}
