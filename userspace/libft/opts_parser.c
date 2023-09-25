/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   opts_parser.c                                      :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2021/12/21 04:59:49 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 19:16:20 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

t_state	opts_parser(char c, t_optable *otb)
{
	static t_parser_state	ps;

	if (otb->init)
	{
		ps = flags;
		otb->init = 0;
	}
	otb->action[ps][(int)c](c, &ps, otb);
	return (o_read);
}
