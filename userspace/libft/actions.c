/* ************************************************************************** */
/*                                                                            */
/*                                                        :::      ::::::::   */
/*   actions.c                                          :+:      :+:    :+:   */
/*                                                    +:+ +:+         +:+     */
/*   By: mypark <mypark@student.42seoul.kr>         +#+  +:+       +#+        */
/*                                                +#+#+#+#+#+   +#+           */
/*   Created: 2022/01/04 01:51:59 by mypark            #+#    #+#             */
/*   Updated: 2022/01/05 20:02:32 by mypark           ###   ########.fr       */
/*                                                                            */
/* ************************************************************************** */

#include "kfs/libft.h"

void append_width(char c, t_parser_state *ps, t_optable *otb) {
	otb->width = (otb->width * 10) + (c - '0');
	*ps = width;
}

void append_preci(char c, t_parser_state *ps, t_optable *otb) {
	otb->preci = (otb->preci * 10) + (c - '0');
	*ps = preci;
}

void goto_width(char c, t_parser_state *ps, t_optable *otb) {
	otb->width = c - '0';
	*ps = width;
}

void goto_preci(char c, t_parser_state *ps, t_optable *otb) {
	c++;
	otb->preci = 0;
	otb->p_chk = 1;
	otb->zero = 0;
	*ps = preci;
}

void fill_optable(char c, t_parser_state *ps, t_optable *otb) {
	if (c == '0' && otb->align == right && !otb->p_chk)
		otb->zero = 1;
	else if (c == ' ' && otb->plus == 0)
		otb->space = 1;
	else if (c == '#')
		otb->base = 1;
	else if (c == '-') {
		otb->align = left;
		otb->zero = 0;
	} else if (c == '+') {
		otb->plus = 1;
		otb->space = 0;
	}
	*ps = flags;
}
